use crate::{
    matching::{resolve_path, PathMatch, RouteDefinition, RouteMatch},
    ParamsMap, RouterContext, SsrMode,
};
use leptos::{leptos_dom::Transparent, *};
use std::{cell::Cell, rc::Rc};

thread_local! {
    static ROUTE_ID: Cell<usize> = Cell::new(0);
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
    tracing::instrument(level = "info", skip_all,)
)]
#[component(transparent)]
pub fn Route<E, F, P>(
    /// The path fragment that this route should match. This can be static (`users`),
    /// include a parameter (`:id`) or an optional parameter (`:id?`), or match a
    /// wildcard (`user/*any`).
    path: P,
    /// The view that should be shown when this route is matched. This can be any function
    /// that returns a type that implements [IntoView] (like `|| view! { <p>"Show this"</p> })`
    /// or `|| view! { <MyComponent/>` } or even, for a component with no props, `MyComponent`).
    view: F,
    /// The mode that this route prefers during server-side rendering. Defaults to out-of-order streaming.
    #[prop(optional)]
    ssr: SsrMode,
    /// The HTTP methods that this route can handle (defaults to only `GET`).
    #[prop(default = &[Method::Get])]
    methods: &'static [Method],
    /// `children` may be empty or include nested routes.
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView
where
    E: IntoView,
    F: Fn() -> E + 'static,
    P: std::fmt::Display,
{
    define_route(
        children,
        path.to_string(),
        Rc::new(move || view().into_view()),
        ssr,
        methods,
    )
}

/// Describes a route that is guarded by a certain condition. This works the same way as
/// [`<Route/>`](Route), except that if the `condition` function evaluates to `false`, it
/// redirects to `redirect_path` instead of displaying its `view`.
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "info", skip_all,)
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
    /// `children` may be empty or include nested routes.
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView
where
    E: IntoView,
    F: Fn() -> E + 'static,
    P: std::fmt::Display + 'static,
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
    )
}
#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "info", skip_all,)
)]
pub(crate) fn define_route(
    children: Option<Children>,
    path: String,
    view: Rc<dyn Fn() -> View>,
    ssr_mode: SsrMode,
    methods: &'static [Method],
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

    let id = ROUTE_ID.with(|id| {
        let next = id.get() + 1;
        id.set(next);
        next
    });

    RouteDefinition {
        id,
        path,
        children,
        view,
        ssr_mode,
        methods,
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
        tracing::instrument(level = "info", skip_all,)
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
            view: element, id, ..
        } = route.key;
        let params = create_memo(move |_| {
            matcher()
                .map(|matched| matched.path_match.params)
                .unwrap_or_default()
        });

        Some(Self {
            inner: Rc::new(RouteContextInner {
                id,
                base_path: base,
                child: Box::new(child),
                path: create_rw_signal(path),
                original_path: route.original_path.to_string(),
                params,
                outlet: Box::new(move || Some(element())),
            }),
        })
    }

    pub(crate) fn id(&self) -> usize {
        self.inner.id
    }

    /// Returns the URL path of the current route,
    /// including param values in their places.
    ///
    /// e.g., this will return `/article/0` rather than `/article/:id`.
    /// For the opposite behavior, see [RouteContext::original_path].
    #[track_caller]
    pub fn path(&self) -> String {
        #[cfg(debug_assertions)]
        let caller = std::panic::Location::caller();

        self.inner.path.try_get_untracked().unwrap_or_else(|| {
            leptos::debug_warn!(
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
    /// For the opposite behavior, see [RouteContext::path].
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
            .map(String::from)
    }

    /// The nested child route, if any.
    pub fn child(&self) -> Option<RouteContext> {
        (self.inner.child)()
    }

    /// The view associated with the current route.
    pub fn outlet(&self) -> impl IntoView {
        (self.inner.outlet)()
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
}

impl PartialEq for RouteContextInner {
    fn eq(&self, other: &Self) -> bool {
        self.base_path == other.base_path
            && self.path == other.path
            && self.original_path == other.original_path
            && self.params == other.params
    }
}

impl std::fmt::Debug for RouteContextInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RouteContextInner")
            .field("path", &self.path)
            .field("ParamsMap", &self.params)
            .finish()
    }
}
