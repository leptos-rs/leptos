use crate::{ResourceId, runtime::PinnedFuture};
use std::collections::{HashMap, HashSet};
use cfg_if::cfg_if;

pub struct SharedContext {
    pub events: Vec<()>,
    pub pending_resources: HashSet<ResourceId>,
    pub resolved_resources: HashMap<ResourceId, String>,
    #[allow(clippy::type_complexity)]
    // index String is the fragment ID: tuple is (ID of previous component, Future of <Suspense/> HTML when resolved)
    pub pending_fragments: HashMap<String, (String, PinnedFuture<String>)>,
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

impl Default for SharedContext {

    fn default() -> Self {
        cfg_if! {
            if #[cfg(feature = "hydrate")] {
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
                    events: Default::default(),
                    pending_resources,
                    resolved_resources,
                    pending_fragments: Default::default(),
                }
            } else {
                Self {
                    events: Default::default(),
                    pending_resources: Default::default(),
                    resolved_resources: Default::default(),
                    pending_fragments: Default::default(),
                }
            }
        }
    }
}
