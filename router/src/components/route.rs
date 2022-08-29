use std::{any::Any, borrow::Cow, rc::Rc};

use leptos_dom::{Child, Element, IntoChild};
use leptos_reactive::{create_memo, Memo, Scope};
use typed_builder::TypedBuilder;

use crate::{
    matching::{resolve_path, PathMatch, RouteDefinition, RouteMatch},
    Action, Loader, ParamsMap, RouterContext,
};

#[derive(TypedBuilder)]
pub struct RouteProps<F, E>
where
    F: Fn(Scope) -> E + 'static,
    E: IntoChild,
{
    path: &'static str,
    element: F,
    #[builder(default, setter(strip_option))]
    loader: Option<Loader>,
    #[builder(default, setter(strip_option))]
    action: Option<Action>,
    #[builder(default)]
    children: Vec<RouteDefinition>,
}

#[allow(non_snake_case)]
pub fn Route<F, E>(cx: Scope, props: RouteProps<F, E>) -> RouteDefinition
where
    F: Fn(Scope) -> E + 'static,
    E: IntoChild,
{
    RouteDefinition {
        path: props.path,
        loader: props.loader,
        action: props.action,
        children: props.children,
        element: Rc::new(move |cx| (props.element)(cx).into_child(cx)),
    }
}

#[derive(Debug, Clone)]
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
        let location = &router.inner.location;
        let base = router.base();
        let base = base.path();
        let RouteMatch { path_match, route } = matcher()?;
        let PathMatch { path, .. } = path_match;
        let RouteDefinition {
            element,
            loader,
            action,
            ..
        } = route.key;
        let params = create_memo(cx, move |_| {
            matcher()
                .map(|matched| matched.path_match.params)
                .unwrap_or_default()
        });

        let data = loader.map(|loader| (loader.data)(cx, params, location.clone()));

        Some(Self {
            inner: Rc::new(RouteContextInner {
                cx,
                base_path: base.to_string(),
                child: Box::new(child),
                data,
                action,
                path,
                original_path: route.original_path.to_string(),
                params,
                outlet: Box::new(move || Some(element(cx))),
            }),
        })
    }

    pub fn cx(&self) -> Scope {
        self.inner.cx
    }

    pub fn path(&self) -> &str {
        &self.inner.path
    }

    pub fn params(&self) -> ParamsMap {
        self.inner.params.get()
    }

    pub fn data(&self) -> &Option<Box<dyn Any>> {
        &self.inner.data
    }

    pub fn base(cx: Scope, path: &str, fallback: Option<fn() -> Element>) -> Self {
        Self {
            inner: Rc::new(RouteContextInner {
                cx,
                base_path: path.to_string(),
                child: Box::new(|| None),
                data: None,
                action: None,
                path: path.to_string(),
                original_path: path.to_string(),
                params: create_memo(cx, |_| ParamsMap::new()),
                outlet: Box::new(move || fallback.map(|f| f().into_child(cx))),
            }),
        }
    }

    pub fn resolve_path<'a>(&'a self, to: &'a str) -> Option<Cow<'a, str>> {
        log::debug!(
            "resolve_path to {to:?}: RouteContext.inner is {:#?}",
            self.inner.path
        );
        resolve_path(&self.inner.base_path, to, Some(&self.inner.path))
    }

    pub(crate) fn child(&self) -> Option<RouteContext> {
        (self.inner.child)()
    }

    pub fn outlet(&self) -> impl IntoChild {
        (self.inner.outlet)()
    }
}

pub(crate) struct RouteContextInner {
    cx: Scope,
    base_path: String,
    pub(crate) child: Box<dyn Fn() -> Option<RouteContext>>,
    pub(crate) data: Option<Box<dyn Any>>,
    pub(crate) action: Option<Action>,
    pub(crate) path: String,
    pub(crate) original_path: String,
    pub(crate) params: Memo<ParamsMap>,
    pub(crate) outlet: Box<dyn Fn() -> Option<Child>>,
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

pub trait IntoChildRoutes {
    fn into_child_routes(self) -> Vec<RouteDefinition>;
}

impl IntoChildRoutes for () {
    fn into_child_routes(self) -> Vec<RouteDefinition> {
        vec![]
    }
}

impl IntoChildRoutes for RouteDefinition {
    fn into_child_routes(self) -> Vec<RouteDefinition> {
        vec![self]
    }
}

impl IntoChildRoutes for Option<RouteDefinition> {
    fn into_child_routes(self) -> Vec<RouteDefinition> {
        self.map(|c| vec![c]).unwrap_or_default()
    }
}

impl IntoChildRoutes for Vec<RouteDefinition> {
    fn into_child_routes(self) -> Vec<RouteDefinition> {
        self
    }
}
