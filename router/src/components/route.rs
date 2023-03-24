use crate::{
    matching::{resolve_path, PathMatch, RouteDefinition, RouteMatch},
    ParamsMap, RouterContext, SsrMode,
};
use leptos::{leptos_dom::Transparent, *};
use std::{cell::Cell, rc::Rc};

thread_local! {
    static ROUTE_ID: Cell<usize> = Cell::new(0);
}

/// Describes a portion of the nested layout of the app, specifying the route it should match,
/// the element it should display, and data that should be loaded alongside the route.
#[component(transparent)]
pub fn Route<E, F, P>(
    cx: Scope,
    /// The path fragment that this route should match. This can be static (`users`),
    /// include a parameter (`:id`) or an optional parameter (`:id?`), or match a
    /// wildcard (`user/*any`).
    path: P,
    /// The view that should be shown when this route is matched. This can be any function
    /// that takes a [Scope] and returns a type that implements [IntoView] (like `|cx| view! { cx, <p>"Show this"</p> })`
    /// or `|cx| view! { cx, <MyComponent/>` } or even, for a component with no props, `MyComponent`).
    view: F,
    /// The mode that this route prefers during server-side rendering. Defaults to out-of-order streaming.
    #[prop(optional)]
    ssr: SsrMode,
    /// `children` may be empty or include nested routes.
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView
where
    E: IntoView,
    F: Fn(Scope) -> E + 'static,
    P: std::fmt::Display,
{
    fn inner(
        cx: Scope,
        children: Option<Children>,
        path: String,
        view: Rc<dyn Fn(Scope) -> View>,
        ssr_mode: SsrMode,
    ) -> RouteDefinition {
        let children = children
            .map(|children| {
                children(cx)
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
        }
    }

    inner(
        cx,
        children,
        path.to_string(),
        Rc::new(move |cx| view(cx).into_view(cx)),
        ssr,
    )
}

impl IntoView for RouteDefinition {
    fn into_view(self, cx: Scope) -> View {
        Transparent::new(self).into_view(cx)
    }
}

/// Context type that contains information about the current, matched route.
#[derive(Debug, Clone, PartialEq)]
pub struct RouteContext {
    inner: Rc<RouteContextInner>,
}

impl RouteContext {
    pub(crate) fn new(
        cx: Scope,
        router: &RouterContext,
        child: impl Fn(Scope) -> Option<RouteContext> + 'static,
        matcher: impl Fn() -> Option<RouteMatch> + 'static,
    ) -> Option<Self> {
        let base = router.base();
        let base = base.path();
        let RouteMatch { path_match, route } = matcher()?;
        let PathMatch { path, .. } = path_match;
        let RouteDefinition {
            view: element, id, ..
        } = route.key;
        let params = create_memo(cx, move |_| {
            matcher()
                .map(|matched| matched.path_match.params)
                .unwrap_or_default()
        });

        Some(Self {
            inner: Rc::new(RouteContextInner {
                cx,
                id,
                base_path: base,
                child: Box::new(child),
                path: create_rw_signal(cx, path),
                original_path: route.original_path.to_string(),
                params,
                outlet: Box::new(move |cx| Some(element(cx))),
            }),
        })
    }

    /// Returns the reactive scope of the current route.
    pub fn cx(&self) -> Scope {
        self.inner.cx
    }

    pub(crate) fn id(&self) -> usize {
        self.inner.id
    }

    /// Returns the URL path of the current route,
    /// including param values in their places.
    ///
    /// e.g., this will return `/article/0` rather than `/article/:id`.
    /// For the opposite behavior, see [RouteContext::original_path].
    pub fn path(&self) -> String {
        self.inner.path.get_untracked()
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

    pub(crate) fn base(
        cx: Scope,
        path: &str,
        fallback: Option<fn(Scope) -> View>,
    ) -> Self {
        Self {
            inner: Rc::new(RouteContextInner {
                cx,
                id: 0,
                base_path: path.to_string(),
                child: Box::new(|_| None),
                path: create_rw_signal(cx, path.to_string()),
                original_path: path.to_string(),
                params: create_memo(cx, |_| ParamsMap::new()),
                outlet: Box::new(move |cx| {
                    fallback.as_ref().map(move |f| f(cx))
                }),
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
    pub fn child(&self, cx: Scope) -> Option<RouteContext> {
        (self.inner.child)(cx)
    }

    /// The view associated with the current route.
    pub fn outlet(&self, cx: Scope) -> impl IntoView {
        (self.inner.outlet)(cx)
    }
}

pub(crate) struct RouteContextInner {
    cx: Scope,
    base_path: String,
    pub(crate) id: usize,
    pub(crate) child: Box<dyn Fn(Scope) -> Option<RouteContext>>,
    pub(crate) path: RwSignal<String>,
    pub(crate) original_path: String,
    pub(crate) params: Memo<ParamsMap>,
    pub(crate) outlet: Box<dyn Fn(Scope) -> Option<View>>,
}

impl PartialEq for RouteContextInner {
    fn eq(&self, other: &Self) -> bool {
        self.cx == other.cx
            && self.base_path == other.base_path
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

#[component]
pub fn ProtectedRoute<P, E, F, C>(
    cx: Scope,
    /// Path that will be exposed if the condition is resolved to true.
    expose_path: P,
    /// Path for the Redirect in case if the condition is resolved to false.
    redirect_path: P,
    /// Condition function that returns a boolean.
    condition: C,
    /// View that will be exposed if the condition is resolved to true.
    view: F,
) -> impl IntoView
where
    E: IntoView,
    F: Fn(Scope) -> E + 'static,
    P: std::fmt::Display + 'static,
    C: Fn(Scope) -> bool + 'static,
{
    if condition(cx) {
        return view! {cx, <Route path=expose_path view=view />}.into_view(cx);
    } else {
        return view! {cx, <Redirect path=redirect_path /> }.into_view(cx);
    }
}
