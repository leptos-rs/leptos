#![forbid(unsafe_code)]
use crate::{runtime::PinnedFuture, suspense::StreamChunk, ResourceId};
use cfg_if::cfg_if;
use std::collections::{HashMap, HashSet, VecDeque};

pub struct SharedContext {
    pub events: Vec<()>,
    pub pending_resources: HashSet<ResourceId>,
    pub resolved_resources: HashMap<ResourceId, String>,
    #[allow(clippy::type_complexity)]
    pub pending_fragments: HashMap<String, FragmentData>,
}

/// Represents its pending `<Suspense/>` fragment.
pub struct FragmentData {
    /// Future that represents how it should be render for an out-of-order stream.
    pub out_of_order: PinnedFuture<String>,
    /// Future that represents how it should be render for an in-order stream.
    pub in_order: PinnedFuture<VecDeque<StreamChunk>>,
    /// Whether the stream should wait for this fragment before sending any data.
    pub should_block: bool,
    /// Future that will resolve when the fragment is ready.
    pub is_ready: Option<PinnedFuture<()>>,
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

#[allow(clippy::derivable_impls)]
impl Default for SharedContext {
    fn default() -> Self {
        cfg_if! {
            if #[cfg(all(feature = "hydrate", target_arch = "wasm32"))] {
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
