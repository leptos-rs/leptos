use std::collections::HashMap;

use crate::Scope;

#[derive(Debug, PartialEq, Eq, Default)]
pub struct SharedContext {
    #[cfg(any(feature = "csr", feature = "hydrate"))]
    pub completed: Vec<web_sys::Element>,
    pub events: Vec<()>,
    pub context: Option<HydrationContext>,
    #[cfg(any(feature = "csr", feature = "hydrate"))]
    pub registry: HashMap<String, web_sys::Element>,
}

impl SharedContext {
    #[cfg(any(feature = "csr", feature = "hydrate"))]
    pub fn new_with_registry(registry: HashMap<String, web_sys::Element>) -> Self {
        Self {
            completed: Default::default(),
            events: Default::default(),
            context: Some(HydrationContext {
                id: "0-".into(),
                count: 0,
            }),
            registry,
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
    count: usize,
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
