use crate::ResourceId;
use std::{
    collections::{HashMap, HashSet},
    future::Future,
    pin::Pin,
};

#[derive(Default)]
pub struct SharedContext {
    pub completed: Vec<web_sys::Element>,
    pub events: Vec<()>,
    pub previous_hydration_key: Option<usize>,
    pub registry: HashMap<String, web_sys::Element>,
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
        self.completed == other.completed
            && self.events == other.events
            && self.previous_hydration_key == other.previous_hydration_key
            && self.registry == other.registry
            && self.pending_resources == other.pending_resources
            && self.resolved_resources == other.resolved_resources
    }
}

impl Eq for SharedContext {}

impl SharedContext {
    #[cfg(feature = "hydrate")]
    pub fn new_with_registry(registry: HashMap<String, web_sys::Element>) -> Self {
        let pending_resources = js_sys::Reflect::get(
            &web_sys::window().unwrap(),
            &wasm_bindgen::JsValue::from_str("__LEPTOS_PENDING_RESOURCES"),
        );
        let pending_resources: HashSet<ResourceId> = pending_resources
            .map_err(|_| ())
            .and_then(|pr| serde_wasm_bindgen::from_value(pr).map_err(|_| ()))
            .unwrap_or_default();

        let resolved_resources = js_sys::Reflect::get(
            &web_sys::window().unwrap(),
            &wasm_bindgen::JsValue::from_str("__LEPTOS_RESOLVED_RESOURCES"),
        )
        .unwrap_or(wasm_bindgen::JsValue::NULL);

        let resolved_resources =
            serde_wasm_bindgen::from_value(resolved_resources).unwrap_or_default();

        Self {
            completed: Default::default(),
            events: Default::default(),
            previous_hydration_key: None,
            registry,
            pending_resources,
            resolved_resources,
            pending_fragments: Default::default(),
        }
    }

    pub fn current_fragment_key(&self) -> String {
        if let Some(id) = &self.previous_hydration_key {
            format!("{}f", id)
        } else {
            "0f".to_string()
        }
    }
}