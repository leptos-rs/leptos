pub use super::{form::*, link::*};
#[cfg(feature = "ssr")]
use crate::location::RequestUrl;
pub use crate::nested_router::Outlet;
use crate::{
    ChooseView, MatchNestedRoutes, NestedRoute, PossibleRouteMatch, RouteDefs,
    SsrMode,
    flat_router::FlatRoutesView,
    hooks::{use_matched, use_navigate},
    location::{
        BrowserUrl, Location, LocationChange, LocationProvider, State, Url,
    },
    navigate::NavigateOptions,
    nested_router::NestedRoutesView,
    resolve_path::resolve_path,
};
use either_of::{Either, EitherOf3};
use leptos::{children, prelude::*};
use or_poisoned::OrPoisoned;
use reactive_graph::{
    computed::suspense::SuspenseContext,
    owner::{Owner, provide_context, use_context},
    signal::ArcRwSignal,
    traits::{GetUntracked, ReadUntracked, Set},
    transition::AsyncTransition,
    wrappers::write::SignalSetter,
};
use std::{
    borrow::Cow,
    fmt::{Debug, Display},
    future::poll_fn,
    mem,
    sync::{Arc, Mutex},
    task::Poll,
    time::Duration,
};
use tachys::reactive_graph::OwnedView;

/// A wrapper that allows passing route definitions as children to a component like [`Routes`],
/// [`FlatRoutes`], [`ParentRoute`], or [`ProtectedParentRoute`].
#[derive(Clone, Debug)]
pub struct RouteChildren<Children>(Children);

impl<Children> RouteChildren<Children> {
    /// Extracts the inner route definition.
    pub fn into_inner(self) -> Children {
        self.0
    }
}

impl<F, Children> ToChildren<F> for RouteChildren<Children>
where
    F: FnOnce() -> Children,
{
    fn to_children(f: F) -> Self {
        RouteChildren(f())
    }
}

#[component(transparent)]
pub fn Router<Chil>(
    /// The base URL for the router. Defaults to `""`.
    #[prop(optional, into)]
    base: Option<Cow<'static, str>>,
    /// A signal that will be set while the navigation process is underway.
    ///
    /// With it, a client-side navigation keeps the previous page on screen until the new
    /// route's view has been created and the resources it created while doing so have loaded;
    /// for a [`ProtectedRoute`], that includes waiting for its `condition` to allow access.
    /// Content that only starts loading once the new page is shown (inside a `Suspend`, for
    /// example) shows its own fallback instead. A navigation that only changes the params of
    /// the route on screen keeps the signal set while the resources that depend on them reload.
    #[prop(optional, into)]
    set_is_routing: Option<SignalSetter<bool>>,
    // TODO trailing slashes
    ///// How trailing slashes should be handled in [`Route`] paths.
    //#[prop(optional)]
    //trailing_slash: TrailingSlash,
    /// The `<Router/>` should usually wrap your whole page. It can contain
    /// any elements, and should include a [`Routes`] component somewhere
    /// to define and display [`Route`]s.
    children: TypedChildren<Chil>,
) -> impl IntoView
where
    Chil: IntoView,
{
    #[cfg(feature = "ssr")]
    let (location_provider, current_url, redirect_hook) = {
        let req = use_context::<RequestUrl>().expect("no RequestUrl provided");
        let parsed = req.parse().expect("could not parse RequestUrl");
        let current_url = ArcRwSignal::new(parsed);

        (None, current_url, Box::new(move |_: &str| {}))
    };

    #[cfg(not(feature = "ssr"))]
    let (location_provider, current_url, redirect_hook) = {
        let owner = Owner::current();
        let location =
            BrowserUrl::new().expect("could not access browser navigation"); // TODO options here
        location.init(base.clone());
        provide_context(location.clone());
        let current_url = location.as_url().clone();

        let redirect_hook = Box::new(move |loc: &str| {
            if let Some(owner) = &owner {
                owner.with(|| BrowserUrl::redirect(loc));
            }
        });

        (Some(location), current_url, redirect_hook)
    };
    // provide router context
    let state = ArcRwSignal::new(State::new(None));
    let location = Location::new(current_url.read_only(), state.read_only());

    // set server function redirect hook
    _ = server_fn::redirect::set_redirect_hook(redirect_hook);

    provide_context(RouterContext {
        base,
        current_url,
        location,
        state,
        set_is_routing,
        query_mutations: Default::default(),
        location_provider,
    });

    let children = children.into_inner();
    children()
}

#[derive(Clone)]
pub(crate) struct RouterContext {
    pub base: Option<Cow<'static, str>>,
    pub current_url: ArcRwSignal<Url>,
    pub location: Location,
    pub state: ArcRwSignal<State>,
    pub set_is_routing: Option<SignalSetter<bool>>,
    pub query_mutations:
        ArcStoredValue<Vec<(Oco<'static, str>, Option<String>)>>,
    pub location_provider: Option<BrowserUrl>,
}

impl RouterContext {
    pub fn navigate(&self, path: &str, options: NavigateOptions) {
        let current = self.current_url.read_untracked();
        let resolved_to = if options.resolve {
            resolve_path(
                self.base.as_deref().unwrap_or_default(),
                path,
                // TODO this should be relative to the current *Route*, I think...
                Some(current.path()),
            )
        } else {
            resolve_path("", path, None)
        };

        let mut url = match BrowserUrl::parse(&resolved_to) {
            Ok(url) => url,
            Err(e) => {
                leptos::logging::error!("Error parsing URL: {e:?}");
                return;
            }
        };
        let query_mutations =
            mem::take(&mut *self.query_mutations.write_value());
        if !query_mutations.is_empty() {
            for (key, value) in query_mutations {
                if let Some(value) = value {
                    url.search_params_mut().replace(key, value);
                } else {
                    url.search_params_mut().remove(&key);
                }
            }
            *url.search_mut() = url
                .search_params()
                .to_query_string()
                .trim_start_matches('?')
                .into()
        }

        if url.origin() != current.origin() {
            window().location().set_href(path).unwrap();
            return;
        }

        // update state signal, if necessary
        if options.state != self.state.get_untracked() {
            self.state.set(options.state.clone());
        }

        // update URL signal, if necessary
        let value = url.to_full_path();
        if current != url {
            drop(current);
            self.current_url.set(url);
        }

        if let Some(location_provider) = &self.location_provider {
            location_provider.complete_navigation(&LocationChange {
                value,
                replace: options.replace,
                scroll: options.scroll,
                state: options.state,
            });
        }
    }

    pub fn resolve_path<'a>(
        &'a self,
        path: &'a str,
        from: Option<&'a str>,
    ) -> Cow<'a, str> {
        let base = self.base.as_deref().unwrap_or_default();
        resolve_path(base, path, from)
    }
}

impl Debug for RouterContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RouterContext")
            .field("base", &self.base)
            .field("current_url", &self.current_url)
            .field("location", &self.location)
            .finish_non_exhaustive()
    }
}

#[component(transparent)]
pub fn Routes<Defs, FallbackFn, Fallback>(
    /// A function that returns the view that should be shown if no route is matched.
    fallback: FallbackFn,
    /// Whether to use the View Transition API during navigation.
    #[prop(optional)]
    transition: bool,
    /// The route definitions. This should consist of one or more [`ParentRoute`] or [`Route`]
    /// components.
    children: RouteChildren<Defs>,
) -> impl IntoView
where
    Defs: MatchNestedRoutes + Clone + Send + 'static,
    FallbackFn: FnOnce() -> Fallback + Clone + Send + 'static,
    Fallback: IntoView + 'static,
{
    let location = use_context::<BrowserUrl>();
    let RouterContext {
        current_url,
        base,
        set_is_routing,
        ..
    } = use_context()
        .expect("<Routes> should be used inside a <Router> component");
    let base = base.map(|base| {
        let mut base = Oco::from(base);
        base.upgrade_inplace();
        base
    });
    let routes = RouteDefs::new_with_base(
        children.into_inner(),
        base.clone().unwrap_or_default(),
    );
    let outer_owner =
        Owner::current().expect("creating Routes, but no Owner was found");
    move || {
        current_url.track();
        outer_owner.with(|| {
            current_url.read_untracked().provide_server_action_error()
        });
        NestedRoutesView {
            location: location.clone(),
            routes: routes.clone(),
            outer_owner: outer_owner.clone(),
            current_url: current_url.clone(),
            base: base.clone(),
            fallback: fallback.clone(),
            set_is_routing,
            transition,
        }
    }
}

#[component(transparent)]
pub fn FlatRoutes<Defs, FallbackFn, Fallback>(
    /// A function that returns the view that should be shown if no route is matched.
    fallback: FallbackFn,
    /// Whether to use the View Transition API during navigation.
    #[prop(optional)]
    transition: bool,
    /// The route definitions. This should consist of one or more [`ParentRoute`] or [`Route`]
    /// components.
    children: RouteChildren<Defs>,
) -> impl IntoView
where
    Defs: MatchNestedRoutes + Clone + Send + 'static,
    FallbackFn: FnOnce() -> Fallback + Clone + Send + 'static,
    Fallback: IntoView + 'static,
{
    let location = use_context::<BrowserUrl>();
    let RouterContext {
        current_url,
        base,
        set_is_routing,
        ..
    } = use_context()
        .expect("<FlatRoutes> should be used inside a <Router> component");

    // TODO base
    #[allow(unused)]
    let base = base.map(|base| {
        let mut base = Oco::from(base);
        base.upgrade_inplace();
        base
    });
    let routes = RouteDefs::new_with_base(
        children.into_inner(),
        base.clone().unwrap_or_default(),
    );

    let outer_owner =
        Owner::current().expect("creating Router, but no Owner was found");

    move || {
        current_url.track();
        outer_owner.with(|| {
            current_url.read_untracked().provide_server_action_error()
        });
        FlatRoutesView {
            current_url: current_url.clone(),
            location: location.clone(),
            routes: routes.clone(),
            fallback: fallback.clone(),
            outer_owner: outer_owner.clone(),
            set_is_routing,
            transition,
        }
    }
}

/// Describes a portion of the nested layout of the app, specifying the route it should match
/// and the element it should display.
#[component(transparent)]
pub fn Route<Segments, View>(
    /// The path fragment that this route should match. This can be created using the
    /// [`path`](crate::path) macro, or path segments ([`StaticSegment`](crate::StaticSegment),
    /// [`ParamSegment`](crate::ParamSegment), [`WildcardSegment`](crate::WildcardSegment), and
    /// [`OptionalParamSegment`](crate::OptionalParamSegment)).
    path: Segments,
    /// The view for this route.
    view: View,
    /// The mode that this route prefers during server-side rendering.
    /// Defaults to out-of-order streaming.
    #[prop(optional)]
    ssr: SsrMode,
) -> <NestedRoute<Segments, (), (), View> as IntoMaybeErased>::Output
where
    View: ChooseView + Clone + 'static,
    Segments: PossibleRouteMatch + Clone + Send + 'static,
{
    NestedRoute::new(path, view)
        .ssr_mode(ssr)
        .into_maybe_erased()
}

/// Describes a portion of the nested layout of the app, specifying the route it should match
/// and the element it should display.
#[component(transparent)]
pub fn ParentRoute<Segments, View, Children>(
    /// The path fragment that this route should match. This can be created using the
    /// [`path`](crate::path) macro, or path segments ([`StaticSegment`](crate::StaticSegment),
    /// [`ParamSegment`](crate::ParamSegment), [`WildcardSegment`](crate::WildcardSegment), and
    /// [`OptionalParamSegment`](crate::OptionalParamSegment)).
    path: Segments,
    /// The view for this route.
    view: View,
    /// Nested child routes.
    children: RouteChildren<Children>,
    /// The mode that this route prefers during server-side rendering.
    /// Defaults to out-of-order streaming.
    #[prop(optional)]
    ssr: SsrMode,
) -> <NestedRoute<Segments, Children, (), View> as IntoMaybeErased>::Output
where
    View: ChooseView + Clone + 'static,
    Children: MatchNestedRoutes + Send + Clone + 'static,
    Segments: PossibleRouteMatch + Clone + Send + 'static,
{
    let children = children.into_inner();
    NestedRoute::new(path, view)
        .ssr_mode(ssr)
        .child(children)
        .into_maybe_erased()
}

/// Describes a route that is guarded by a certain condition. This works the same way as
/// [`<Route/>`], except that if the `condition` function evaluates to `Some(false)`, it
/// redirects to `redirect_path` instead of displaying its `view`.
///
/// With [`<Router set_is_routing>`](Router), a navigation to this route keeps the previous
/// page on screen, as it does for a [`<Route/>`], until the `view` has been created and the
/// resources it creates have loaded: it waits for `condition` to allow access first, if
/// `condition` reads a resource that is still loading. A `condition` that is `None` without
/// waiting for a resource cannot be waited for: the route then shows its `fallback`.
#[component(transparent)]
pub fn ProtectedRoute<Segments, ViewFn, View, C, PathFn, P>(
    /// The path fragment that this route should match. This can be created using the
    /// [`path`](crate::path) macro, or path segments ([`StaticSegment`](crate::StaticSegment),
    /// [`ParamSegment`](crate::ParamSegment), [`WildcardSegment`](crate::WildcardSegment), and
    /// [`OptionalParamSegment`](crate::OptionalParamSegment)).
    path: Segments,
    /// The view for this route.
    view: ViewFn,
    /// A function that returns `Option<bool>`, where `Some(true)` means that the user can access
    /// the page, `Some(false)` means the user cannot access the page, and `None` means this
    /// information is still loading.
    condition: C,
    /// The path that will be redirected to if the condition is `Some(false)`.
    redirect_path: PathFn,
    /// Will be displayed while the condition is pending. By default this is the empty view.
    #[prop(optional, into)]
    fallback: children::ViewFn,
    /// The mode that this route prefers during server-side rendering.
    /// Defaults to out-of-order streaming.
    #[prop(optional)]
    ssr: SsrMode,
) -> ProtectedRouteOutput<Segments, (), ViewFn, C, PathFn>
where
    Segments: PossibleRouteMatch + Clone + Send + 'static,
    ViewFn: Fn() -> View + Send + Clone + 'static,
    View: IntoView + 'static,
    C: Fn() -> Option<bool> + Send + Clone + 'static,
    PathFn: Fn() -> P + Send + Clone + 'static,
    P: Display + 'static,
{
    let view = ProtectedRouteView {
        view,
        condition,
        redirect_path,
        fallback,
        parent: false,
    };
    NestedRoute::new(path, view)
        .ssr_mode(ssr)
        .into_maybe_erased()
}

/// Describes a route with nested child routes that is guarded by a certain condition. This
/// works the same way as [`<ParentRoute/>`](ParentRoute), except that if the `condition`
/// function evaluates to `Some(false)`, it redirects to `redirect_path` instead of displaying
/// its `view`.
///
/// With [`<Router set_is_routing>`](Router), a navigation to this route waits for `condition`
/// and for its `view` like a [`<ProtectedRoute/>`] does.
#[component(transparent)]
pub fn ProtectedParentRoute<Segments, ViewFn, View, C, PathFn, P, Children>(
    /// The path fragment that this route should match. This can be created using the
    /// [`path`](crate::path) macro, or path segments ([`StaticSegment`](crate::StaticSegment),
    /// [`ParamSegment`](crate::ParamSegment), [`WildcardSegment`](crate::WildcardSegment), and
    /// [`OptionalParamSegment`](crate::OptionalParamSegment)).
    path: Segments,
    /// The view for this route.
    view: ViewFn,
    /// A function that returns `Option<bool>`, where `Some(true)` means that the user can access
    /// the page, `Some(false)` means the user cannot access the page, and `None` means this
    /// information is still loading.
    condition: C,
    /// Will be displayed while the condition is pending. By default this is the empty view.
    #[prop(optional, into)]
    fallback: children::ViewFn,
    /// The path that will be redirected to if the condition is `Some(false)`.
    redirect_path: PathFn,
    /// Nested child routes.
    children: RouteChildren<Children>,
    /// The mode that this route prefers during server-side rendering.
    /// Defaults to out-of-order streaming.
    #[prop(optional)]
    ssr: SsrMode,
) -> ProtectedRouteOutput<Segments, Children, ViewFn, C, PathFn>
where
    Segments: PossibleRouteMatch + Clone + Send + 'static,
    Children: MatchNestedRoutes + Send + Clone + 'static,
    ViewFn: Fn() -> View + Send + Clone + 'static,
    View: IntoView + 'static,
    C: Fn() -> Option<bool> + Send + Clone + 'static,
    PathFn: Fn() -> P + Send + Clone + 'static,
    P: Display + 'static,
{
    let children = children.into_inner();
    let view = ProtectedRouteView {
        view,
        condition,
        redirect_path,
        fallback,
        parent: true,
    };
    NestedRoute::new(path, view)
        .ssr_mode(ssr)
        .child(children)
        .into_maybe_erased()
}

/// The route that [`ProtectedRoute`] and [`ProtectedParentRoute`] describe.
type ProtectedRouteOutput<Segments, Children, ViewFn, C, PathFn> =
    <NestedRoute<Segments, Children, (), ProtectedRouteView<ViewFn, C, PathFn>> as IntoMaybeErased>::Output;

/// The view of a [`ProtectedRoute`] or a [`ProtectedParentRoute`].
///
/// It shows the route's view once `condition` allows it, through a `<Transition>`. When it is
/// chosen during a navigation that waits for the new route (with `set_is_routing`), it first
/// waits for `condition`, and creates the view right away if access is granted, so that the
/// navigation waits for the resources the view creates too.
#[doc(hidden)]
#[derive(Clone)]
pub struct ProtectedRouteView<ViewFn, C, PathFn> {
    view: ViewFn,
    condition: C,
    redirect_path: PathFn,
    fallback: children::ViewFn,
    // a ProtectedParentRoute creates its view under the owner of its outlet,
    // not one nested within the <Transition>, so that the context it provides
    // reaches the views of its child routes
    parent: bool,
}

impl<ViewFn, View, C, PathFn, P> ChooseView
    for ProtectedRouteView<ViewFn, C, PathFn>
where
    ViewFn: Fn() -> View + Send + Clone + 'static,
    View: IntoView + 'static,
    C: Fn() -> Option<bool> + Send + Clone + 'static,
    PathFn: Fn() -> P + Send + Clone + 'static,
    P: Display + 'static,
{
    async fn choose(self) -> AnyView {
        // a navigation waits for the resources created while the view is
        // chosen only if it chooses it in an async transition
        if AsyncTransition::is_active()
            && wait_for_condition(&self.condition).await == Some(true)
        {
            let view = self.view.clone();
            let seed = if self.parent {
                Seed {
                    owner: None,
                    view: view(),
                }
            } else {
                let owner = Owner::new();
                let view = owner.with(view);
                Seed {
                    owner: Some(owner),
                    view,
                }
            };
            self.seeded_view(seed)
        } else {
            self.into_view()
        }
    }

    async fn preload(&self) {}
}

impl<ViewFn, View, C, PathFn, P> ProtectedRouteView<ViewFn, C, PathFn>
where
    ViewFn: Fn() -> View + Send + Clone + 'static,
    View: IntoView + 'static,
    C: Fn() -> Option<bool> + Send + Clone + 'static,
    PathFn: Fn() -> P + Send + Clone + 'static,
    P: Display + 'static,
{
    /// The route's view, which creates the protected view whenever it shows it.
    fn into_view(self) -> AnyView {
        let view = self.view;
        if self.parent {
            let owner = Owner::current().expect("no current reactive Owner");
            guarded(
                self.condition,
                self.redirect_path,
                self.fallback,
                move || owner.with(&view),
            )
        } else {
            guarded(self.condition, self.redirect_path, self.fallback, view)
        }
    }

    /// The route's view, which shows the protected view created while it was
    /// chosen the first time it shows the protected view.
    fn seeded_view(self, seed: Seed<View>) -> AnyView {
        let seed = Arc::new(SeedSlot(Mutex::new(Some(seed))));
        let view = self.view;
        let owner = self
            .parent
            .then(|| Owner::current().expect("no current reactive Owner"));
        guarded(
            self.condition,
            self.redirect_path,
            self.fallback,
            move || {
                match seed.take() {
                    Some(Seed {
                        owner: Some(owner),
                        view,
                    }) => {
                        // give the view the context it would have had, had
                        // it been created here: the resources it reads while
                        // rendering then suspend this route's <Transition>
                        if let Some(suspense) = use_context::<SuspenseContext>()
                        {
                            owner.with(|| provide_context(suspense));
                        }
                        // and dispose of it along with this branch, like a
                        // view created here
                        Owner::on_cleanup({
                            let owner = owner.clone();
                            move || owner.cleanup()
                        });
                        Either::Left(OwnedView::new_with_owner(view, owner))
                    }
                    Some(Seed { owner: None, view }) => Either::Right(view),
                    None => Either::Right(match &owner {
                        Some(owner) => owner.with(&view),
                        None => view(),
                    }),
                }
            },
        )
    }
}

/// Shows `content` if `condition` is `Some(true)`, the fallback while it is
/// `None`, and redirects to `redirect_path` if it is `Some(false)`.
fn guarded<C, PathFn, P, Content, V>(
    condition: C,
    redirect_path: PathFn,
    fallback: children::ViewFn,
    content: Content,
) -> AnyView
where
    C: Fn() -> Option<bool> + Send + Clone + 'static,
    PathFn: Fn() -> P + Send + Clone + 'static,
    P: Display + 'static,
    Content: Fn() -> V + Send + Clone + 'static,
    V: IntoView + 'static,
{
    let fallback = move || fallback.run();
    (view! {
        <Transition fallback=fallback.clone()>
            {move || {
                let condition = condition();
                let content = content.clone();
                let redirect_path = redirect_path.clone();
                let fallback = fallback.clone();
                Unsuspend::new(move || match condition {
                    Some(true) => EitherOf3::A(content()),
                    #[allow(clippy::unit_arg)]
                    Some(false) => {
                        EitherOf3::B(view! { <Redirect path=redirect_path()/> }.into_inner())
                    }
                    None => EitherOf3::C(fallback()),
                })
            }}

        </Transition>
    })
    .into_any()
}

/// The protected view of a [`ProtectedRouteView`], created while it was chosen.
struct Seed<View> {
    // `None` for a ProtectedParentRoute, whose view is created under the
    // owner of its outlet
    owner: Option<Owner>,
    view: View,
}

/// Holds a [`Seed`] until it is shown, and disposes of it if it never is.
struct SeedSlot<View>(Mutex<Option<Seed<View>>>);

impl<View> SeedSlot<View> {
    fn take(&self) -> Option<Seed<View>> {
        self.0.lock().or_poisoned().take()
    }
}

impl<View> Drop for SeedSlot<View> {
    fn drop(&mut self) {
        if let Some(Seed {
            owner: Some(owner), ..
        }) = self.take()
        {
            owner.cleanup();
        }
    }
}

/// How many times [`wait_for_condition`] evaluates a condition that is waiting
/// for resources: once it has read one that has loaded, a condition may read
/// another one that is still loading.
const CONDITION_ROUNDS: usize = 8;

/// Evaluates `condition`, and while it is `None` because a resource it reads is
/// still loading, waits for that resource and evaluates it again.
async fn wait_for_condition(
    condition: &impl Fn() -> Option<bool>,
) -> Option<bool> {
    for _ in 0..CONDITION_ROUNDS {
        // a resource read under a SuspenseContext registers a task with it
        // until it has loaded
        let suspense = SuspenseContext::default();
        let owner = Owner::new();
        let value = owner.with(|| {
            provide_context(suspense.clone());
            untrack(condition)
        });
        owner.cleanup();
        if value.is_some() {
            return value;
        }
        let mut loading = false;
        poll_fn(|cx| {
            if suspense.poll_empty(cx.waker()) {
                Poll::Ready(())
            } else {
                loading = true;
                Poll::Pending
            }
        })
        .await;
        if !loading {
            return None;
        }
    }
    None
}

/// Redirects the user to a new URL, whether on the client side or on the server
/// side. If rendered on the server, this sets a `302` status code if `permanent` is false or a `301` if `permanent` is true,
/// and sets a `Location` header. If rendered in the browser, it uses client-side navigation to redirect.
/// In either case, it resolves the route relative to the current route. (To use
/// an absolute path, prefix it with `/`).
///
/// **Note**: Support for server-side redirects is provided by the server framework
/// integrations ([`leptos_actix`] and [`leptos_axum`]. If you’re not using one of those
/// integrations, you should manually provide a way of redirecting on the server
/// using [`provide_server_redirect`].
///
/// [`leptos_actix`]: <https://docs.rs/leptos_actix/>
/// [`leptos_axum`]: <https://docs.rs/leptos_axum/>
#[component(transparent)]
pub fn Redirect<P>(
    /// The relative path to which the user should be redirected.
    path: P,
    /// Navigation options to be used on the client side.
    #[prop(optional)]
    #[allow(unused)]
    options: Option<NavigateOptions>,
    /// Permanent redirect
    ///
    /// Only used on the server side to set a `301` if `permanent` is true
    /// or a `302` if `permanent` is false (`false` by default)
    #[prop(optional)]
    #[allow(unused)]
    permanent: bool,
) where
    P: core::fmt::Display + 'static,
{
    // TODO resolve relative path
    let path = path.to_string();

    // redirect on the server
    if let Some(redirect_fn) = use_context::<ServerRedirectFunction>() {
        (redirect_fn.f)(
            &resolve_path("", &path, Some(&use_matched().get_untracked())),
            permanent,
        );
    }
    // redirect on the client
    else {
        if cfg!(feature = "ssr") {
            #[cfg(feature = "tracing")]
            tracing::warn!(
                "Calling <Redirect/> without a ServerRedirectFunction \
                 provided, in SSR mode."
            );

            #[cfg(not(feature = "tracing"))]
            eprintln!(
                "Calling <Redirect/> without a ServerRedirectFunction \
                 provided, in SSR mode."
            );
            return;
        }
        let navigate = use_navigate();
        navigate(&path, options.unwrap_or_default());
    }
}

type ServerRedirectDynFunction = dyn Fn(&str, bool) + Send + Sync;
/// Wrapping type for a function provided as context to allow for
/// server-side redirects. See [`provide_server_redirect`]
/// and [`Redirect`].
#[derive(Clone)]
pub struct ServerRedirectFunction {
    f: Arc<ServerRedirectDynFunction>,
}

impl core::fmt::Debug for ServerRedirectFunction {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ServerRedirectFunction").finish()
    }
}

/// Provides a function that can be used to redirect the user to another absolute path, on the server.
/// The passed function takes 2 arguments:
/// - `path` (`&str`): the path to redirect the client to
/// - `permanent` (`bool`): indicate if this is a permanent redirect
///
/// The passed function should set a `302` status code if `permanent` is false, or a `301` status code if `permanent` is true,
/// and set an appropriate `Location` header.
pub fn provide_server_redirect(
    handler: impl Fn(&str, bool) + Send + Sync + 'static,
) {
    provide_context(ServerRedirectFunction {
        f: Arc::new(handler),
    })
}

/// A visible indicator that the router is in the process of navigating
/// to another route.
///
/// This is used when `<Router set_is_routing>` has been provided, to
/// provide some visual indicator that the page is currently loading
/// async data, so that it is does not appear to have frozen. It can be
/// styled independently.
#[component]
pub fn RoutingProgress(
    /// Whether the router is currently loading the new page.
    #[prop(into)]
    is_routing: Signal<bool>,
    /// The maximum expected time for loading, which is used to
    /// calibrate the animation process.
    #[prop(optional, into)]
    max_time: std::time::Duration,
    /// The time to show the full progress bar after page has loaded, before hiding it. (Defaults to 100ms.)
    #[prop(default = std::time::Duration::from_millis(250))]
    before_hiding: std::time::Duration,
) -> impl IntoView {
    const INCREMENT_EVERY_MS: f32 = 5.0;
    let expected_increments =
        max_time.as_secs_f32() / (INCREMENT_EVERY_MS / 1000.0);
    // `max_time` is optional and defaults to `Duration::ZERO`, which would make
    // `expected_increments` zero and `100.0 / 0.0` evaluate to `inf` (and then
    // `NaN`), producing `width: NaN%`. Fill the bar in a single increment when
    // no positive `max_time` was provided.
    let percent_per_increment = if expected_increments > 0.0 {
        100.0 / expected_increments
    } else {
        100.0
    };

    let (is_showing, set_is_showing) = signal(false);
    let (progress, set_progress) = signal(0.0);

    StoredValue::new(RenderEffect::new(
        move |prev: Option<Option<IntervalHandle>>| {
            if is_routing.get() && !is_showing.get() {
                set_is_showing.set(true);
                set_interval(
                    move || {
                        set_progress.update(|n| *n += percent_per_increment);
                    },
                    Duration::from_millis(INCREMENT_EVERY_MS as u64),
                )
                .ok()
            } else if is_routing.get() && is_showing.get() {
                set_progress.set(0.0);
                prev?
            } else {
                set_progress.set(100.0);
                _ = set_timeout(
                    move || {
                        set_progress.set(0.0);
                        set_is_showing.set(false);
                    },
                    before_hiding,
                );
                if let Some(Some(interval)) = prev {
                    interval.clear();
                }
                None
            }
        },
    ));

    view! {
        <Show when=is_showing>
            <progress min="0" max="100" value=move || progress.get()></progress>
        </Show>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::FutureExt;

    /// Without a navigation that waits for the new route (with server-side
    /// rendering, hydration or the initial load, or without `set_is_routing`),
    /// a protected route's view is chosen at once, as it was before it could
    /// wait for its condition.
    #[test]
    fn protected_route_view_is_chosen_at_once_outside_a_transition() {
        let owner = Owner::new();
        owner.set();
        for parent in [false, true] {
            let view = ProtectedRouteView {
                view: || "content",
                condition: || None::<bool>,
                redirect_path: || "/",
                fallback: children::ViewFn::default(),
                parent,
            };
            assert!(view.choose().now_or_never().is_some());
        }
    }
}
