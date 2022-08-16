use std::borrow::Cow;

pub struct RouteContext {}

impl RouteContext {
    pub fn new(path: &str) -> Self {
        Self {}
    }

    pub fn resolve_path(&self, to: &str) -> Option<Cow<str>> {
        todo!()
    }
}
