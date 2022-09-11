#[cfg(any(feature = "hydrate"))]
use std::collections::{HashMap, HashSet};

#[cfg(any(feature = "hydrate"))]
use crate::{Scope, StreamingResourceId};

#[derive(Debug, PartialEq, Eq, Default)]
pub struct SharedContext {
    #[cfg(any(feature = "hydrate"))]
    pub completed: Vec<web_sys::Element>,
    pub events: Vec<()>,
    pub context: Option<HydrationContext>,
    #[cfg(any(feature = "hydrate"))]
    pub registry: HashMap<String, web_sys::Element>,
    #[cfg(any(feature = "hydrate"))]
    pub pending_resources: HashSet<StreamingResourceId>,
    #[cfg(any(feature = "hydrate"))]
    pub resolved_resources: HashMap<StreamingResourceId, String>,
}

impl SharedContext {
    #[cfg(feature = "hydrate")]
    pub fn new_with_registry(registry: HashMap<String, web_sys::Element>) -> Self {
        let pending_resources = js_sys::Reflect::get(
            &web_sys::window().unwrap(),
            &wasm_bindgen::JsValue::from_str("__LEPTOS_PENDING_RESOURCES"),
        );
        let pending_resources: HashSet<StreamingResourceId> = pending_resources
            .map_err(|_| ())
            .and_then(|pr| serde_wasm_bindgen::from_value(pr).map_err(|_| ()))
            .unwrap_or_default();

        let resolved_resources = js_sys::Reflect::get(
            &web_sys::window().unwrap(),
            &wasm_bindgen::JsValue::from_str("__LEPTOS_RESOLVED_RESOURCES"),
        )
        .unwrap_or(wasm_bindgen::JsValue::NULL);
        log::debug!(
            "(create_resource) (hydration.rs) resolved resources from JS = {:#?}",
            resolved_resources
        );
        /*  let resolved_resources = resolved_resources
        .map_err(|_| ())
        .and_then(|pr| serde_wasm_bindgen::from_value(pr).map_err(|_| ()))
        .unwrap_or_default(); */
        let resolved_resources = match serde_wasm_bindgen::from_value(resolved_resources) {
            Ok(v) => v,
            Err(e) => {
                log::debug!(
                    "(create_resource) error deserializing __LEPTOS_RESOLVED_RESOURCES\n\n{e}"
                );
                HashMap::default()
            }
        };
        log::debug!(
            "(create_resource) (hydration.rs) resolved resources after deserializing = {:#?}",
            resolved_resources
        );

        Self {
            completed: Default::default(),
            events: Default::default(),
            context: Some(HydrationContext {
                id: "0-".into(),
                count: -1,
            }),
            registry,
            pending_resources,
            resolved_resources,
        }
    }

    pub fn next_hydration_key(&mut self) -> String {
        if let Some(context) = &mut self.context {
            context.count += 1;
            format!("{}{}", context.id, context.count)
        } else {
            self.context = Some(HydrationContext {
                id: "0-".into(),
                count: 0,
            });
            "0-0".into()
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
