use std::{borrow::Cow, rc::Rc};

use leptos::*;
use typed_builder::TypedBuilder;

use crate::{
    matching::{resolve_path, PathMatch, RouteDefinition, RouteMatch},
    ParamsMap, RouterContext,
};

/// Properties that can be passed to a [Route] component, which describes
/// a portion of the nested layout of the app, specifying the route it should match,
/// the element it should display, and data that should be loaded alongside the route.
#[derive(TypedBuilder)]
pub struct RouteProps<E, F>
where
    E: IntoView,
    F: Fn(Scope) -> E + 'static,
{
    /// The path fragment that this route should match. This can be static (`users`),
    /// include a parameter (`:id`) or an optional parameter (`:id?`), or match a
    /// wildcard (`user/*any`).
    pub path: &'static str,
    /// The view that should be shown when this route is matched. This can be any function
    /// that takes a [Scope] and returns an [Element] (like `|cx| view! { cx, <p>"Show this"</p> })`
    /// or `|cx| view! { cx, <MyComponent/>` } or even, for a component with no props, `MyComponent`).
    pub element: F,
    /// `children` may be empty or include nested routes.
    #[builder(default, setter(strip_option))]
    pub children: Option<Box<dyn Fn() -> Vec<RouteDefinition>>>,
}

/// Describes a portion of the nested layout of the app, specifying the route it should match,
/// the element it should display, and data that should be loaded alongside the route.
#[allow(non_snake_case)]
pub fn Route<E, F>(_cx: Scope, props: RouteProps<E, F>) -> RouteDefinition
where
    E: IntoView,
    F: Fn(Scope) -> E + 'static,
{
    RouteDefinition {
        path: props.path,
        children: props.children.map(|c| c()).unwrap_or_default(),
        element: Rc::new(move |cx| (props.element)(cx).into_view(cx)),
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
        let RouteDefinition { element, .. } = route.key;
        let params = create_memo(cx, move |_| {
            matcher()
                .map(|matched| matched.path_match.params)
                .unwrap_or_default()
        });

        Some(Self {
            inner: Rc::new(RouteContextInner {
                cx,
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

    pub(crate) fn base(cx: Scope, path: &str, fallback: Option<fn() -> View>) -> Self {
        Self {
            inner: Rc::new(RouteContextInner {
                cx,
                base_path: path.to_string(),
                child: Box::new(|| None),
                path: path.to_string(),
                original_path: path.to_string(),
                params: create_memo(cx, |_| ParamsMap::new()),
                outlet: Box::new(move || fallback.map(|f| f().into_view(cx))),
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
