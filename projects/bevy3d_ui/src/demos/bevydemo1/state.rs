use bevy::ecs::system::Resource;
use std::sync::{Arc, Mutex};

pub type Shared<T> = Arc<Mutex<T>>;

/// Shared Resource used for Bevy
#[derive(Resource)]
pub struct SharedResource(pub Shared<SharedState>);

/// Shared State
pub struct SharedState {
    pub name: String,
}

impl SharedState {
    /// Get a new shared state
    pub fn new() -> Arc<Mutex<SharedState>> {
        let state = SharedState {
            name: "This can be used for shared state".to_string(),
        };
        let shared = Arc::new(Mutex::new(state));
        shared
    }
}
