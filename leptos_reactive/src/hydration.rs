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
    pub context: Option<HydrationContext>,
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
            && self.context == other.context
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
            context: Some(HydrationContext {
                id: "".into(),
                count: -1,
            }),
            registry,
            pending_resources,
            resolved_resources,
            pending_fragments: Default::default(),
        }
    }

    pub fn next_hydration_key(&mut self) -> String {
        if let Some(context) = &mut self.context {
            let k = format!("{}{}", context.id, context.count);
            context.count += 1;
            k
        } else {
            self.context = Some(HydrationContext {
                id: "0-".into(),
                count: 1,
            });
            "0-0".into()
        }
    }

    #[cfg(feature = "ssr")]
    pub fn current_fragment_key(&self) -> String {
        if let Some(context) = &self.context {
            format!("{}{}f", context.id, context.count)
        } else {
            "0f".to_string()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct HydrationContext {
    id: String,
    count: i32,
}

impl HydrationContext {
    pub fn next_hydration_context(&mut self) -> HydrationContext {
        self.count += 1;
        HydrationContext {
            id: format!("{}{}-", self.id, self.count),
            count: 0,
        }
    }
}
