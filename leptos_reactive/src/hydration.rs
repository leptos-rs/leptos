use crate::ResourceId;
use std::{
    collections::{HashMap, HashSet},
    future::Future,
    pin::Pin,
};

#[derive(Default)]
pub struct SharedContext {
    pub events: Vec<()>,
    pub pending_resources: HashSet<ResourceId>,
    pub resolved_resources: HashMap<ResourceId, String>,
    pub pending_fragments: HashMap<String, Pin<Box<dyn Future<Output = String>>>>,
}

impl std::fmt::Debug for SharedContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedContext").finish()
    }
}

impl PartialEq for SharedContext {
    fn eq(&self, other: &Self) -> bool {
        self.events == other.events
            && self.pending_resources == other.pending_resources
            && self.resolved_resources == other.resolved_resources
    }
}

impl Eq for SharedContext {}

impl SharedContext {

}