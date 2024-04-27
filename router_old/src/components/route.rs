use crate::{
    matching::{resolve_path, PathMatch, RouteDefinition, RouteMatch},
    ParamsMap, RouterContext, SsrMode, StaticData, StaticMode, StaticParamsMap,
    TrailingSlash,
};
use leptos::{leptos_dom::Transparent, *};
use std::{
    any::Any,
    borrow::Cow,
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::Arc,
};

thread_local! {
    static ROUTE_ID: Cell<usize> = const { Cell::new(0) };
}

// RouteDefinition.id is `pub` and required to be unique.
// Should we make this public so users can generate unique IDs?
pub(in crate::components) fn new_route_id() -> usize {
    ROUTE_ID.with(|id| {
        let next = id.get() + 1;
        id.set(next);
        next
    })
}

/// Represents an HTTP method that can be handled by this route.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum Method {
    /// The [`GET`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods/GET) method
    /// requests a representation of the specified resource.
    #[default]
    Get,
    /// The [`POST`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods/POST) method
    /// submits an entity to the specified resource, often causing a change in
    /// state or side effects on the server.
    Post,
    /// The [`PUT`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods/PUT) method
    /// replaces all current representations of the target resource with the request payload.
    Put,
    /// The [`DELETE`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods/DELETE) method
    /// deletes the specified resource.
    Delete,
    /// The [`PATCH`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods/PATCH) method
    /// applies partial modifications to a resource.
    Patch,
}

/// Describes a portion of the nested layout of the app, specifying the route it should match,
/// the element it should display, and data that should be loaded alongside the route.
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "trace", skip_all,)
)]
#[component(transparent)]
pub fn Route<E, F, P>(
    /// The path fragment that this route should match. This can be static (`users`),
    /// include a parameter (`:id`) or an optional parameter (`:id?`), or match a
    /// wildcard (`user/*any`).
    path: P,
    /// The view that should be shown when this route is matched. This can be any function
    /// that returns a type that implements [`IntoView`] (like `|| view! { <p>"Show this"</p> })`
    /// or `|| view! { <MyComponent/>` } or even, for a component with no props, `MyComponent`).
    view: F,
    /// The mode that this route prefers during server-side rendering. Defaults to out-of-order streaming.
    #[prop(optional)]
    ssr: SsrMode,
    /// The HTTP methods that this route can handle (defaults to only `GET`).
    #[prop(default = &[Method::Get])]
    methods: &'static [Method],
    /// A data-loading function that will be called when the route is matched. Its results can be
    /// accessed with [`use_route_data`](crate::use_route_data).
    #[prop(optional, into)]
    data: Option<Loader>,
    /// How this route should handle trailing slashes in its path.
    /// Overrides any setting applied to [`crate::components::Router`].
    /// Serves as a default for any inner Routes.
    #[prop(optional)]
    trailing_slash: Option<TrailingSlash>,
    /// `children` may be empty or include nested routes.
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView
where
    E: IntoView,
    F: Fn() -> E + 'static,
    P: core::fmt::Display,
{
    define_route(
        children,
        path.to_string(),
        Rc::new(move || view().into_view()),
        ssr,
        methods,
        data,
        None,
        None,
        trailing_slash,
    )
}

/// Describes a route that is guarded by a certain condition. This works the same way as
/// [`<Route/>`](Route), except that if the `condition` function evaluates to `false`, it
/// redirects to `redirect_path` instead of displaying its `view`.
///
/// ## Reactive or Asynchronous Conditions
///
/// Note that the condition check happens once, at the time of navigation to the page. It
/// is not reactive (i.e., it will not cause the user to navigate away from the page if the
/// condition changes to `false`), which means it does not work well with asynchronous conditions.
/// If you need to protect a route conditionally or via `Suspense`, you should used nested routing
/// and wrap the condition around the `<Outlet/>`.
///
/// ```rust
/// # use leptos::*; use leptos_router::*;
/// # if false {
/// let has_permission = move || true; // TODO!
///
/// view! {
///  <Routes>
///    // parent route
///    <Route path="/" view=move || {
///      view! {
///        // only show the outlet when `has_permission` is `true`, and hide it when it is `false`
///        <Show when=move || has_permission() fallback=|| "Access denied!">
///          <Outlet/>
///        </Show>
///      }
///    }>
///      // nested child route
///      <Route path="/" view=|| view! { <p>"Protected data" </p> }/>
///    </Route>
///  </Routes>
/// }
/// # ;}
/// ```
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "trace", skip_all,)
)]
#[component(transparent)]
pub fn ProtectedRoute<P, E, F, C>(
    /// The path fragment that this route should match. This can be static (`users`),
    /// include a parameter (`:id`) or an optional parameter (`:id?`), or match a
    /// wildcard (`user/*any`).
    path: P,
    /// The path that will be redirected to if the condition is `false`.
    redirect_path: P,
    /// Condition function that returns a boolean.
    condition: C,
    /// View that will be exposed if the condition is `true`.
    view: F,
    /// The mode that this route prefers during server-side rendering. Defaults to out-of-order streaming.
    #[prop(optional)]
    ssr: SsrMode,
    /// The HTTP methods that this route can handle (defaults to only `GET`).
    #[prop(default = &[Method::Get])]
    methods: &'static [Method],
    /// A data-loading function that will be called when the route is matched. Its results can be
    /// accessed with [`use_route_data`](crate::use_route_data).
    #[prop(optional, into)]
    data: Option<Loader>,
    /// How this route should handle trailing slashes in its path.
    /// Overrides any setting applied to [`crate::components::Router`].
    /// Serves as a default for any inner Routes.
    #[prop(optional)]
    trailing_slash: Option<TrailingSlash>,
    /// `children` may be empty or include nested routes.
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView
where
    E: IntoView,
    F: Fn() -> E + 'static,
    P: core::fmt::Display + 'static,
    C: Fn() -> bool + 'static,
{
    use crate::Redirect;
    let redirect_path = redirect_path.to_string();

    define_route(
        children,
        path.to_string(),
        Rc::new(move || {
            if condition() {
                view().into_view()
            } else {
                view! { <Redirect path=redirect_path.clone()/> }.into_view()
            }
        }),
        ssr,
        methods,
        data,
        None,
        None,
        trailing_slash,
    )
}

/// Describes a portion of the nested layout of the app, specifying the route it should match,
/// the element it should display, and data that should be loaded alongside the route.
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "trace", skip_all,)
)]
#[component(transparent)]
pub fn StaticRoute<E, F, P, S>(
    /// The path fragment that this route should match. This can be static (`users`),
    /// include a parameter (`:id`) or an optional parameter (`:id?`), or match a
    /// wildcard (`user/*any`).
    path: P,
    /// The view that should be shown when this route is matched. This can be any function
    /// that returns a type that implements [IntoView] (like `|| view! { <p>"Show this"</p> })`
    /// or `|| view! { <MyComponent/>` } or even, for a component with no props, `MyComponent`).
    view: F,
    /// Creates a map of the params that should be built for a particular route.
    static_params: S,
    /// The static route mode
    #[prop(optional)]
    mode: StaticMode,
    /// A data-loading function that will be called when the route is matched. Its results can be
    /// accessed with [`use_route_data`](crate::use_route_data).
    #[prop(optional, into)]
    data: Option<Loader>,
    /// How this route should handle trailing slashes in its path.
    /// Overrides any setting applied to [`crate::components::Router`].
    /// Serves as a default for any inner Routes.
    #[prop(optional)]
    trailing_slash: Option<TrailingSlash>,
    /// `children` may be empty or include nested routes.
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView
where
    E: IntoView,
    F: Fn() -> E + 'static,
    P: core::fmt::Display,
    S: Fn() -> Pin<Box<dyn Future<Output = StaticParamsMap> + Send + Sync>>
        + Send
        + Sync
        + 'static,
{
    define_route(
        children,
        path.to_string(),
        Rc::new(move || view().into_view()),
        SsrMode::default(),
        &[Method::Get],
        data,
        Some(mode),
        Some(Arc::new(static_params)),
        trailing_slash,
    )
}

#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "trace", skip_all,)
)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn define_route(
    children: Option<Children>,
    path: String,
    view: Rc<dyn Fn() -> View>,
    ssr_mode: SsrMode,
    methods: &'static [Method],
    data: Option<Loader>,
    static_mode: Option<StaticMode>,
    static_params: Option<StaticData>,
    trailing_slash: Option<TrailingSlash>,
) -> RouteDefinition {
    let children = children
        .map(|children| {
            children()
                .as_children()
                .iter()
                .filter_map(|child| {
                    child
                        .as_transparent()
                        .and_then(|t| t.downcast_ref::<RouteDefinition>())
                })
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    RouteDefinition {
        id: new_route_id(),
        path,
        children,
        view,
        ssr_mode,
        methods,
        data,
        static_mode,
        static_params,
        trailing_slash,
    }
}

impl IntoView for RouteDefinition {
    fn into_view(self) -> View {
        Transparent::new(self).into_view()
    }
}

/// Context type that contains information about the current, matched route.
#[derive(Debug, Clone, PartialEq)]
pub struct RouteContext {
    pub(crate) inner: Rc<RouteContextInner>,
}

impl RouteContext {
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        tracing::instrument(level = "trace", skip_all,)
    )]
    pub(crate) fn new(
        router: &RouterContext,
        child: impl Fn() -> Option<RouteContext> + 'static,
        matcher: impl Fn() -> Option<RouteMatch> + 'static,
    ) -> Option<Self> {
        let base = router.base();
        let base = base.path();
        let RouteMatch { path_match, route } = matcher()?;
        let PathMatch { path, .. } = path_match;
        let RouteDefinition {
            view: element,
            id,
            data,
            ..
        } = route.key;
        let params = create_memo(move |_| {
            matcher()
                .map(|matched| matched.path_match.params)
                .unwrap_or_default()
        });

        let inner = Rc::new(RouteContextInner {
            id,
            base_path: base,
            child: Box::new(child),
            path: create_rw_signal(path),
            original_path: route.original_path.to_string(),
            params,
            outlet: Box::new(move || Some(element())),
            data: RefCell::new(None),
        });
        if let Some(loader) = data {
            let data = {
                let inner = Rc::clone(&inner);
                provide_context(RouteContext { inner });
                (loader.data)()
            };
            *inner.data.borrow_mut() = Some(data);
        }

        Some(RouteContext { inner })
    }

    pub(crate) fn id(&self) -> usize {
        self.inner.id
    }

    /// Returns the URL path of the current route,
    /// including param values in their places.
    ///
    /// e.g., this will return `/article/0` rather than `/article/:id`.
    /// For the opposite behavior, see [`RouteContext::original_path`].
    #[track_caller]
    pub fn path(&self) -> String {
        #[cfg(debug_assertions)]
        let caller = std::panic::Location::caller();

        self.inner.path.try_get_untracked().unwrap_or_else(|| {
            leptos::logging::debug_warn!(
                "at {caller}, you call `.path()` on a `<Route/>` that has \
                 already been disposed"
            );
            Default::default()
        })
    }

    pub(crate) fn set_path(&self, path: String) {
        self.inner.path.set(path);
    }

    /// Returns the original URL path of the current route,
    /// with the param name rather than the matched parameter itself.
    ///
    /// e.g., this will return `/article/:id` rather than `/article/0`
    /// For the opposite behavior, see [`RouteContext::path`].
    pub fn original_path(&self) -> &str {
        &self.inner.original_path
    }

    /// A reactive wrapper for the route parameters that are currently matched.
    pub fn params(&self) -> Memo<ParamsMap> {
        self.inner.params
    }

    pub(crate) fn base(path: &str, fallback: Option<fn() -> View>) -> Self {
        Self {
            inner: Rc::new(RouteContextInner {
                id: 0,
                base_path: path.to_string(),
                child: Box::new(|| None),
                path: create_rw_signal(path.to_string()),
                original_path: path.to_string(),
                params: create_memo(|_| ParamsMap::new()),
                outlet: Box::new(move || fallback.as_ref().map(move |f| f())),
                data: Default::default(),
            }),
        }
    }

    /// Resolves a relative route, relative to the current route's path.
    pub fn resolve_path(&self, to: &str) -> Option<String> {
        resolve_path(
            &self.inner.base_path,
            to,
            Some(&self.inner.path.get_untracked()),
        )
        .map(String::from)
    }

    pub(crate) fn resolve_path_tracked(&self, to: &str) -> Option<String> {
        resolve_path(&self.inner.base_path, to, Some(&self.inner.path.get()))
            .map(Cow::into_owned)
    }

    /// The nested child route, if any.
    pub fn child(&self) -> Option<RouteContext> {
        (self.inner.child)()
    }

    /// The view associated with the current route.
    pub fn outlet(&self) -> impl IntoView {
        (self.inner.outlet)()
    }

    /// The http method used to navigate to this route. Defaults to [`Method::Get`] when unavailable like in client side routing
    pub fn method(&self) -> Method {
        use_context().unwrap_or_default()
    }
}

pub(crate) struct RouteContextInner {
    base_path: String,
    pub(crate) id: usize,
    pub(crate) child: Box<dyn Fn() -> Option<RouteContext>>,
    pub(crate) path: RwSignal<String>,
    pub(crate) original_path: String,
    pub(crate) params: Memo<ParamsMap>,
    pub(crate) outlet: Box<dyn Fn() -> Option<View>>,
    pub(crate) data: RefCell<Option<Rc<dyn Any>>>,
}

impl PartialEq for RouteContextInner {
    fn eq(&self, other: &Self) -> bool {
        self.base_path == other.base_path
            && self.path == other.path
            && self.original_path == other.original_path
            && self.params == other.params
    }
}

impl core::fmt::Debug for RouteContextInner {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RouteContextInner")
            .field("path", &self.path)
            .field("ParamsMap", &self.params)
            .finish()
    }
}

#[derive(Clone)]
pub struct Loader {
    pub(crate) data: Rc<dyn Fn() -> Rc<dyn Any>>,
}

impl<F, T> From<F> for Loader
where
    F: Fn() -> T + 'static,
    T: Any + Clone + 'static,
{
    fn from(f: F) -> Self {
        Self {
            data: Rc::new(move || Rc::new(f())),
        }
    }
}
