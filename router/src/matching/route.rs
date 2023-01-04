use std::rc::Rc;

use leptos::leptos_dom::View;
use leptos::*;

#[derive(Clone)]
pub struct RouteDefinition {
    pub id: usize,
    pub path: String,
    pub children: Vec<RouteDefinition>,
    pub view: Rc<dyn Fn(Scope) -> View>,
}

impl std::fmt::Debug for RouteDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RouteDefinition")
            .field("path", &self.path)
            .field("children", &self.children)
            .finish()
    }
}

impl PartialEq for RouteDefinition {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.children == other.children
    }
}
