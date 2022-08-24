use std::{any::Any, borrow::Cow, rc::Rc};

use leptos_dom::{Child, Element, IntoChild};
use leptos_reactive::Scope;
use typed_builder::TypedBuilder;

use crate::{
    matching::{PathMatch, RouteDefinition, RouteMatch},
    Action, Loader, ParamsMap, RouteData, RouterContext,
};

#[derive(TypedBuilder)]
pub struct RouteProps<F, E>
where
    F: Fn() -> E + 'static,
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
    F: Fn() -> E + 'static,
    E: IntoChild,
{
    RouteDefinition {
        path: props.path,
        loader: props.loader,
        action: props.action,
        children: props.children,
        element: Rc::new(move || (props.element)().into_child(cx)),
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
        matcher: impl Fn() -> RouteMatch,
    ) -> Self {
        let location = &router.inner.location;
        let RouteMatch { path_match, route } = matcher();
        let RouteDefinition {
            element,
            loader,
            action,
            ..
        } = route.key;
        let PathMatch { path, params } = path_match;

        let data = loader.map(|loader| {
            let data = (loader.data)(cx, params.clone(), location.clone());
            log::debug!(
                "RouteContext: set data to {:?}\n\ntype ID is {:?}",
                data,
                data.type_id()
            );
            data
        });

        Self {
            inner: Rc::new(RouteContextInner {
                child: Box::new(child),
                data,
                action,
                path,
                params,
                outlet: Box::new(move || Some(element())),
            }),
        }
    }

    pub fn params(&self) -> &ParamsMap {
        &self.inner.params
    }

    pub fn data(&self) -> &Option<Box<dyn Any>> {
        &self.inner.data
    }

    pub fn base(cx: Scope, path: &str, fallback: Option<fn() -> Element>) -> Self {
        Self {
            inner: Rc::new(RouteContextInner {
                child: Box::new(|| None),
                data: None,
                action: None,
                path: path.to_string(),
                params: ParamsMap::new(),
                outlet: Box::new(move || fallback.map(|f| f().into_child(cx))),
            }),
        }
    }

    pub fn resolve_path(&self, to: &str) -> Option<Cow<str>> {
        log::debug!("RouteContext::resolve_path");
        todo!()
    }

    pub(crate) fn child(&self) -> Option<RouteContext> {
        (self.inner.child)()
    }

    pub fn outlet(&self) -> impl IntoChild {
        log::debug!("looking for outlet A");
        let o = (self.inner.outlet)();
        log::debug!("outlet = {o:#?}");
        o
    }
}

pub(crate) struct RouteContextInner {
    pub(crate) child: Box<dyn Fn() -> Option<RouteContext>>,
    pub(crate) data: Option<Box<dyn Any>>,
    pub(crate) action: Option<Action>,
    pub(crate) path: String,
    pub(crate) params: ParamsMap,
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
