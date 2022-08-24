use std::{
    any::Any,
    borrow::Cow,
    fmt::Debug,
    rc::{Rc, Weak},
};

use leptos_dom::{Child, IntoChild};

use crate::{DataFunction, ParamsMap, PathMatch, Route, RouteMatch, RouterContext};

#[derive(Debug, Clone)]
pub struct RouteDefinition {
    pub path: Vec<String>,
    pub data: Option<DataFunction>,
    pub children: Vec<RouteDefinition>,
    pub component: Child,
}
#[derive(Debug, Clone)]
pub struct RouteContext {
    pub(crate) inner: Rc<RouteContextInner>,
}

pub(crate) struct RouteContextInner {
    pub(crate) parent: Option<RouteContext>,
    pub(crate) get_child: Box<dyn Fn() -> Option<RouteContext>>,
    pub(crate) data: Option<Box<dyn Any>>,
    pub(crate) path: String,
    pub(crate) params: ParamsMap,
    pub(crate) outlet: Box<dyn Fn() -> Option<Child>>,
}

impl Debug for RouteContextInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RouteContextInner")
            .field("parent", &self.parent)
            .field("path", &self.path)
            .field("ParamsMap", &self.params)
            .finish()
    }
}

pub(crate) fn create_route_context(
    router: &RouterContext,
    parent: &RouteContext,
    child: impl Fn() -> Option<RouteContext> + 'static,
    matcher: impl Fn() -> RouteMatch,
) -> RouteContext {
    let location = &router.inner.location;
    let base = &router.inner.base;
    let RouteMatch { path_match, route } = matcher();
    let component = route.key.component.clone();
    let PathMatch { path, params } = path_match;
    log::debug!("in create_route_context, params = {params:?}");
    let Route {
        key,
        pattern,
        original_path,
        matcher,
    } = route;
    let get_child = Box::new(child);

    RouteContext {
        inner: Rc::new(RouteContextInner {
            parent: Some(parent.clone()),
            get_child,
            data: None, // TODO route data,
            path,
            params,
            outlet: Box::new(move || Some(component.clone())),
        }),
    }
}

impl RouteContext {
    pub fn base(path: &str) -> Self {
        Self {
            inner: Rc::new(RouteContextInner {
                parent: None,
                get_child: Box::new(|| None),
                data: None,
                path: path.to_string(),
                params: ParamsMap::new(),
                outlet: Box::new(|| None),
            }),
        }
    }

    pub fn resolve_path(&self, to: &str) -> Option<Cow<str>> {
        log::debug!("RouteContext::resolve_path");
        todo!()
    }

    pub(crate) fn child(&self) -> Option<RouteContext> {
        (self.inner.get_child)()
    }

    pub fn outlet(&self) -> impl IntoChild {
        log::debug!("looking for outlet A");
        let o = (self.inner.outlet)();
        log::debug!("outlet = {o:#?}");
        o
    }
}

impl PartialEq for RouteDefinition {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
            && self.children == other.children
            && self.component == other.component
    }
}
