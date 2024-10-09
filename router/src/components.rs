pub use super::{form::*, link::*};
#[cfg(feature = "ssr")]
use crate::location::RequestUrl;
pub use crate::nested_router::Outlet;
use crate::{
    flat_router::FlatRoutesView,
    hooks::use_navigate,
    location::{
        BrowserUrl, Location, LocationChange, LocationProvider, State, Url,
    },
    navigate::NavigateOptions,
    nested_router::NestedRoutesView,
    resolve_path::resolve_path,
    ChooseView, MatchNestedRoutes, NestedRoute, Routes, SsrMode,
};
use either_of::Either;
use leptos::prelude::*;
use reactive_graph::{
    owner::{provide_context, use_context, Owner},
    signal::ArcRwSignal,
    traits::{GetUntracked, ReadUntracked, Set},
    wrappers::write::SignalSetter,
};
use std::{
    borrow::Cow,
    fmt::{Debug, Display},
    mem,
    sync::Arc,
    time::Duration,
};
use tachys::view::any_view::AnyView;

#[derive(Debug)]
pub struct RouteChildren<Children>(Children);

impl<Children> RouteChildren<Children> {
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
    #[prop(optional, into)]
    set_is_routing: Option<SignalSetter<bool>>,
    // TODO trailing slashes
    ///// How trailing slashes should be handled in [`Route`] paths.
    //#[prop(optional)]
    //trailing_slash: TrailingSlash,
    /// The `<Router/>` should usually wrap your whole page. It can contain
    /// any elements, and should include a [`Routes`](crate::Routes) component somewhere
    /// to define and display [`Route`](crate::Route)s.
    children: TypedChildren<Chil>,
) -> impl IntoView
where
    Chil: IntoView,
{
    #[cfg(feature = "ssr")]
    let (current_url, redirect_hook) = {
        let req = use_context::<RequestUrl>().expect("no RequestUrl provided");
        let parsed = req.parse().expect("could not parse RequestUrl");
        let current_url = ArcRwSignal::new(parsed);

        (current_url, Box::new(move |_: &str| {}))
    };

    #[cfg(not(feature = "ssr"))]
    let (current_url, redirect_hook) = {
        let location =
            BrowserUrl::new().expect("could not access browser navigation"); // TODO options here
        location.init(base.clone());
        provide_context(location.clone());
        let current_url = location.as_url().clone();

        let redirect_hook = Box::new(|loc: &str| BrowserUrl::redirect(loc));

        (current_url, redirect_hook)
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

        let mut url = match resolved_to.map(|to| BrowserUrl::parse(&to)) {
            Some(Ok(url)) => url,
            Some(Err(e)) => {
                leptos::logging::error!("Error parsing URL: {e:?}");
                return;
            }
            None => {
                leptos::logging::error!("Error resolving relative URL.");
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

        BrowserUrl::complete_navigation(&LocationChange {
            value,
            replace: options.replace,
            scroll: options.scroll,
            state: options.state,
        });
    }

    pub fn resolve_path<'a>(
        &'a self,
        path: &'a str,
        from: Option<&'a str>,
    ) -> Option<Cow<'a, str>> {
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

/*
#[component]
pub fn FlatRouter<Children, FallbackFn, Fallback>(
    #[prop(optional, into)] base: Option<Cow<'static, str>>,
    fallback: FallbackFn,
    children: RouteChildren<Children>,
) -> FlatRouter<Dom, BrowserUrl, Children, FallbackFn>
where
    FallbackFn: Fn() -> Fallback,
{
    let children = Routes::new(children.into_inner());
    if let Some(base) = base {
        FlatRouter::new_with_base(base, children, fallback)
    } else {
        FlatRouter::new(children, fallback)
    }
}*/

#[component(transparent)]
pub fn Routes<Defs, FallbackFn, Fallback>(
    fallback: FallbackFn,
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
    let routes = Routes::new(children.into_inner());
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
        }
    }
}

#[component(transparent)]
pub fn FlatRoutes<Defs, FallbackFn, Fallback>(
    fallback: FallbackFn,
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
    let routes = Routes::new(children.into_inner());

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
        }
    }
}

#[component(transparent)]
pub fn Route<Segments, View>(
    path: Segments,
    view: View,
    #[prop(optional)] ssr: SsrMode,
) -> NestedRoute<Segments, (), (), View>
where
    View: ChooseView,
{
    NestedRoute::new(path, view).ssr_mode(ssr)
}

#[component(transparent)]
pub fn ParentRoute<Segments, View, Children>(
    path: Segments,
    view: View,
    children: RouteChildren<Children>,
    #[prop(optional)] ssr: SsrMode,
) -> NestedRoute<Segments, Children, (), View>
where
    View: ChooseView,
{
    let children = children.into_inner();
    NestedRoute::new(path, view).ssr_mode(ssr).child(children)
}

#[component(transparent)]
pub fn ProtectedRoute<Segments, ViewFn, View, C, PathFn, P>(
    path: Segments,
    view: ViewFn,
    condition: C,
    redirect_path: PathFn,
    #[prop(optional)] ssr: SsrMode,
) -> NestedRoute<Segments, (), (), impl Fn() -> AnyView + Send + Clone>
where
    ViewFn: Fn() -> View + Send + Clone + 'static,
    View: IntoView + 'static,
    C: Fn() -> Option<bool> + Send + Clone + 'static,
    PathFn: Fn() -> P + Send + Clone + 'static,
    P: Display + 'static,
{
    let view = move || {
        let condition = condition.clone();
        let redirect_path = redirect_path.clone();
        let view = view.clone();
        (view! {
            <Transition>
                {move || {
                    let condition = condition();
                    let view = view.clone();
                    let redirect_path = redirect_path.clone();
                    Unsuspend::new(move || match condition {
                        Some(true) => Either::Left(view()),
                        #[allow(clippy::unit_arg)]
                        Some(false) => {
                            Either::Right(view! { <Redirect path=redirect_path()/> }.into_inner())
                        }
                        None => Either::Right(()),
                    })
                }}

            </Transition>
        })
        .into_any()
    };
    NestedRoute::new(path, view).ssr_mode(ssr)
}

#[component(transparent)]
pub fn ProtectedParentRoute<Segments, ViewFn, View, C, PathFn, P, Children>(
    path: Segments,
    view: ViewFn,
    condition: C,
    redirect_path: PathFn,
    children: RouteChildren<Children>,
    #[prop(optional)] ssr: SsrMode,
) -> NestedRoute<Segments, Children, (), impl Fn() -> AnyView + Send + Clone>
where
    ViewFn: Fn() -> View + Send + Clone + 'static,
    View: IntoView + 'static,
    C: Fn() -> Option<bool> + Send + Clone + 'static,
    PathFn: Fn() -> P + Send + Clone + 'static,
    P: Display + 'static,
{
    let children = children.into_inner();
    let view = move || {
        let condition = condition.clone();
        let redirect_path = redirect_path.clone();
        let view = view.clone();
        let owner = Owner::current().unwrap();
        let view = {
            move || {
                let condition = condition();
                let view = view.clone();
                let redirect_path = redirect_path.clone();
                let owner = owner.clone();
                Unsuspend::new(move || match condition {
                    // reset the owner so that things like providing context work
                    // otherwise, this will be a child owner nested within the Transition, not
                    // the parent owner of the Outlet
                    //
                    // clippy: not redundant, a FnOnce vs FnMut issue
                    #[allow(clippy::redundant_closure)]
                    Some(true) => Either::Left(owner.with(|| view())),
                    #[allow(clippy::unit_arg)]
                    Some(false) => Either::Right(
                        view! { <Redirect path=redirect_path()/> }.into_inner(),
                    ),
                    None => Either::Right(()),
                })
            }
        };
        (view! { <Transition>{view}</Transition> }).into_any()
    };
    NestedRoute::new(path, view).ssr_mode(ssr).child(children)
}

/// Redirects the user to a new URL, whether on the client side or on the server
/// side. If rendered on the server, this sets a `302` status code and sets a `Location`
/// header. If rendered in the browser, it uses client-side navigation to redirect.
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
) where
    P: core::fmt::Display + 'static,
{
    // TODO resolve relative path
    let path = path.to_string();

    // redirect on the server
    if let Some(redirect_fn) = use_context::<ServerRedirectFunction>() {
        (redirect_fn.f)(&path);
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

/// Wrapping type for a function provided as context to allow for
/// server-side redirects. See [`provide_server_redirect`]
/// and [`Redirect`].
#[derive(Clone)]
pub struct ServerRedirectFunction {
    f: Arc<dyn Fn(&str) + Send + Sync>,
}

impl core::fmt::Debug for ServerRedirectFunction {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ServerRedirectFunction").finish()
    }
}

/// Provides a function that can be used to redirect the user to another
/// absolute path, on the server. This should set a `302` status code and an
/// appropriate `Location` header.
pub fn provide_server_redirect(handler: impl Fn(&str) + Send + Sync + 'static) {
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
    let percent_per_increment = 100.0 / expected_increments;

    let (is_showing, set_is_showing) = signal(false);
    let (progress, set_progress) = signal(0.0);

    StoredValue::new(RenderEffect::new(
        move |prev: Option<Option<IntervalHandle>>| {
            if is_routing.get() && !is_showing.get() {
                set_is_showing.set(true);
                set_interval_with_handle(
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
                set_timeout(
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
        <Show when=move || is_showing.get() fallback=|| ()>
            <progress min="0" max="100" value=move || progress.get()></progress>
        </Show>
    }
}
