use crate::{Loader, Method, SsrMode, StaticData, StaticMode, TrailingSlash};
use leptos::leptos_dom::View;
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
    pub view: Rc<dyn Fn() -> View>,
    /// The mode this route prefers during server-side rendering.
    pub ssr_mode: SsrMode,
    /// The HTTP request methods this route is able to handle.
    pub methods: &'static [Method],
    /// A data loader function that will be called when this route is matched.
    pub data: Option<Loader>,
    /// The route's preferred mode of static generation, if any
    pub static_mode: Option<StaticMode>,
    /// The data required to fill any dynamic segments in the path during static rendering.
    pub static_params: Option<StaticData>,
    /// How a trailng slash in `path` should be handled.
    pub trailing_slash: Option<TrailingSlash>,
}

impl core::fmt::Debug for RouteDefinition {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RouteDefinition")
            .field("path", &self.path)
            .field("children", &self.children)
            .field("ssr_mode", &self.ssr_mode)
            .field("static_render", &self.static_mode)
            .field("trailing_slash", &self.trailing_slash)
            .finish()
    }
}

impl PartialEq for RouteDefinition {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.children == other.children
    }
}
