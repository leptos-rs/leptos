use crate::{
    hooks::use_navigate,
    location::{
        BrowserUrl, Location, LocationChange, LocationProvider, RequestUrl,
        State, Url,
    },
    navigate::{NavigateOptions, UseNavigate},
    params::ParamsMap,
    resolve_path::resolve_path,
    FlatRoutesView, MatchNestedRoutes, NestedRoute, NestedRoutesView, Routes,
    SsrMode,
};
use leptos::prelude::*;
use reactive_graph::{
    computed::ArcMemo,
    owner::{provide_context, use_context, Owner},
    signal::{ArcRwSignal, RwSignal},
    traits::{GetUntracked, Read, ReadUntracked, Set},
    untrack,
};
use std::{borrow::Cow, fmt::Debug, marker::PhantomData, sync::Arc};
use tachys::renderer::{dom::Dom, Renderer};

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

#[component]
pub fn Router<Chil>(
    /// The base URL for the router. Defaults to `""`.
    #[prop(optional, into)]
    base: Option<Cow<'static, str>>,

    // TODO these prop
    ///// A fallback that should be shown if no route is matched.
    //#[prop(optional)]
    //fallback: Option<fn() -> View>,
    ///// A signal that will be set while the navigation process is underway.
    //#[prop(optional, into)]
    //set_is_routing: Option<SignalSetter<bool>>,
    ///// How trailing slashes should be handled in [`Route`] paths.
    //#[prop(optional)]
    //trailing_slash: TrailingSlash,
    /// The `<Router/>` should usually wrap your whole page. It can contain
    /// any elements, and should include a [`Routes`](crate::Routes) component somewhere
    /// to define and display [`Route`](crate::Route)s.
    children: TypedChildren<Chil>,
    /// A unique identifier for this router, allowing you to mount multiple Leptos apps with
    /// different routes from the same server.
    #[prop(optional)]
    id: usize,
) -> impl IntoView
where
    Chil: IntoView,
{
    #[cfg(feature = "ssr")]
    let current_url = {
        let req = use_context::<RequestUrl>().expect("no RequestUrl provided");
        let parsed = req.parse().expect("could not parse RequestUrl");
        ArcRwSignal::new(parsed)
    };

    #[cfg(not(feature = "ssr"))]
    let current_url = {
        let location =
            BrowserUrl::new().expect("could not access browser navigation"); // TODO options here
        location.init(base.clone());
        provide_context(location.clone());
        location.as_url().clone()
    };
    // provide router context
    let state = ArcRwSignal::new(State::new(None));
    let location = Location::new(current_url.read_only(), state.read_only());

    // TODO server function redirect hook

    provide_context(RouterContext {
        base,
        current_url,
        location,
        state,
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

        let url = match resolved_to.map(|to| BrowserUrl::parse(&to)) {
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

        // update state signal, if necessary
        if options.state != self.state.get_untracked() {
            self.state.set(options.state.clone());
        }

        // update URL signal, if necessary
        if current != url {
            drop(current);
            self.current_url.set(url);
        }

        BrowserUrl::complete_navigation(&LocationChange {
            value: path.to_string(),
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

#[component]
pub fn Routes<Defs, FallbackFn, Fallback>(
    fallback: FallbackFn,
    children: RouteChildren<Defs>,
) -> impl IntoView
where
    Defs: MatchNestedRoutes<Dom> + Clone + Send + 'static,
    FallbackFn: Fn() -> Fallback + Send + 'static,
    Fallback: IntoView + 'static,
{
    let location = use_context::<BrowserUrl>();
    let RouterContext {
        current_url, base, ..
    } = use_context()
        .expect("<Routes> should be used inside a <Router> component");
    let base = base.map(|base| {
        let mut base = Oco::from(base);
        base.upgrade_inplace();
        base
    });
    let routes = Routes::new(children.into_inner());
    let path = ArcMemo::new({
        let url = current_url.clone();
        move |_| url.read().path().to_string()
    });
    let search_params = ArcMemo::new({
        let url = current_url.clone();
        move |_| url.read().search_params().clone()
    });
    let outer_owner =
        Owner::current().expect("creating Routes, but no Owner was found");
    move || NestedRoutesView {
        location: location.clone(),
        routes: routes.clone(),
        outer_owner: outer_owner.clone(),
        url: current_url.clone(),
        path: path.clone(),
        search_params: search_params.clone(),
        base: base.clone(),
        fallback: fallback(),
        rndr: PhantomData,
    }
}

#[component]
pub fn FlatRoutes<Defs, FallbackFn, Fallback>(
    fallback: FallbackFn,
    children: RouteChildren<Defs>,
) -> impl IntoView
where
    Defs: MatchNestedRoutes<Dom> + Clone + Send + 'static,
    FallbackFn: Fn() -> Fallback + Send + 'static,
    Fallback: IntoView + 'static,
{
    let location = use_context::<BrowserUrl>();
    let RouterContext {
        current_url, base, ..
    } = use_context()
        .expect("<FlatRoutes> should be used inside a <Router> component");
    let base = base.map(|base| {
        let mut base = Oco::from(base);
        base.upgrade_inplace();
        base
    });
    let routes = Routes::new(children.into_inner());
    let path = ArcMemo::new({
        let url = current_url.clone();
        move |_| url.read().path().to_string()
    });
    let search_params = ArcMemo::new({
        let url = current_url.clone();
        move |_| url.read().search_params().clone()
    });
    let outer_owner =
        Owner::current().expect("creating Router, but no Owner was found");
    let params = ArcRwSignal::new(ParamsMap::new());
    move || {
        path.track();
        FlatRoutesView {
            location: location.clone(),
            routes: routes.clone(),
            path: path.clone(),
            fallback: fallback(),
            outer_owner: outer_owner.clone(),
            params: params.clone(),
        }
    }
}

#[component]
pub fn Route<Segments, View, ViewFn>(
    path: Segments,
    view: ViewFn,
    #[prop(optional)] ssr: SsrMode,
) -> NestedRoute<Segments, (), (), ViewFn, Dom>
where
    ViewFn: Fn() -> View,
{
    NestedRoute::new(path, view, ssr)
}

#[component]
pub fn ParentRoute<Segments, View, Children, ViewFn>(
    path: Segments,
    view: ViewFn,
    children: RouteChildren<Children>,
    #[prop(optional)] ssr: SsrMode,
) -> NestedRoute<Segments, Children, (), ViewFn, Dom>
where
    ViewFn: Fn() -> View,
{
    let children = children.into_inner();
    NestedRoute::new(path, view, ssr).child(children)
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
#[component]
pub fn Redirect<P>(
    /// The relative path to which the user should be redirected.
    path: P,
    /// Navigation options to be used on the client side.
    #[prop(optional)]
    #[allow(unused)]
    options: Option<NavigateOptions>,
) -> impl IntoView
where
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
