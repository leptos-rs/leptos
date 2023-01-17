use std::{borrow::Cow, cell::Cell, rc::Rc};

use leptos::*;

use crate::{
    matching::{resolve_path, PathMatch, RouteDefinition, RouteMatch},
    ParamsMap, RouterContext,
};

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
    /// that takes a [Scope] and returns an [Element] (like `|cx| view! { cx, <p>"Show this"</p> })`
    /// or `|cx| view! { cx, <MyComponent/>` } or even, for a component with no props, `MyComponent`).
    view: F,
    /// `children` may be empty or include nested routes.
    #[prop(optional)]
    children: Option<Box<dyn FnOnce(Scope) -> Fragment>>,
) -> impl IntoView
where
    E: IntoView,
    F: Fn(Scope) -> E + 'static,
    P: std::fmt::Display,
{
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
        path: path.to_string(),
        children,
        view: Rc::new(move |cx| view(cx).into_view(cx)),
    }
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
        let params = create_memo(cx, move |_| {
            matcher()
                .map(|matched| matched.path_match.params)
                .unwrap_or_default()
        });

        Some(Self {
            inner: Rc::new(RouteContextInner {
                cx,
                id,
                base_path: base.to_string(),
                child: Box::new(child),
                path,
                original_path: route.original_path.to_string(),
                params,
                outlet: Box::new(move || Some(element(cx))),
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
    pub fn path(&self) -> &str {
        &self.inner.path
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

    pub(crate) fn base(cx: Scope, path: &str, fallback: Option<fn(Scope) -> View>) -> Self {
        Self {
            inner: Rc::new(RouteContextInner {
                cx,
                id: 0,
                base_path: path.to_string(),
                child: Box::new(|| None),
                path: path.to_string(),
                original_path: path.to_string(),
                params: create_memo(cx, |_| ParamsMap::new()),
                outlet: Box::new(move || fallback.as_ref().map(move |f| f(cx))),
            }),
        }
    }

    /// Resolves a relative route, relative to the current route's path.
    pub fn resolve_path<'a>(&'a self, to: &'a str) -> Option<Cow<'a, str>> {
        resolve_path(&self.inner.base_path, to, Some(&self.inner.path))
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
    cx: Scope,
    base_path: String,
    pub(crate) id: usize,
    pub(crate) child: Box<dyn Fn() -> Option<RouteContext>>,
    pub(crate) path: String,
    pub(crate) original_path: String,
    pub(crate) params: Memo<ParamsMap>,
    pub(crate) outlet: Box<dyn Fn() -> Option<View>>,
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
            .field("child", &(self.child)())
            .finish()
    }
}
