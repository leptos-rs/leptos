use crate::{Method, SsrMode};
use leptos::{leptos_dom::View, *};
use std::rc::Rc;

/// Defines a single route in a nested route tree. This is the return
/// type of the [`<Route/>`](crate::Route) component, but can also be
/// used to build your own configuration-based or filesystem-based routing.
#[derive(Clone)]
pub struct RouteDefinition {
    /// A unique ID for each route.
    pub id: usize,
    /// The path. This can include params like `:id` or wildcards like `*all`.
    pub path: String,
    /// Other route definitions nested within this one.
    pub children: Vec<RouteDefinition>,
    /// The view that should be displayed when this route is matched.
    pub view: Rc<dyn Fn(Scope) -> View>,
    /// The mode this route prefers during server-side rendering.
    pub ssr_mode: SsrMode,
    /// The HTTP request methods this route is able to handle.
    pub methods: &'static [Method],
}

impl std::fmt::Debug for RouteDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RouteDefinition")
            .field("path", &self.path)
            .field("children", &self.children)
            .field("ssr_mode", &self.ssr_mode)
            .finish()
    }
}

impl PartialEq for RouteDefinition {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.children == other.children
    }
}
