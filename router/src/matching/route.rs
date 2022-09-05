use std::rc::Rc;

use leptos_dom::Child;
use leptos_reactive::Scope;

use crate::{Action, Loader};

#[derive(Clone)]
pub struct RouteDefinition {
    pub path: &'static str,
    pub loader: Option<Loader>,
    pub action: Option<Action>,
    pub children: Vec<RouteDefinition>,
    pub element: Rc<dyn Fn(Scope) -> Child>,
}

impl std::fmt::Debug for RouteDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RouteDefinition")
            .field("path", &self.path)
            .field("loader", &self.loader)
            .field("action", &self.action)
            .field("children", &self.children)
            .finish()
    }
}

impl PartialEq for RouteDefinition {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.children == other.children
    }
}

impl Default for RouteDefinition {
    fn default() -> Self {
        Self {
            path: Default::default(),
            loader: Default::default(),
            action: Default::default(),
            children: Default::default(),
            element: Rc::new(|_| Child::Null),
        }
    }
}
