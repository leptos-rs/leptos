use std::collections::HashMap;

#[derive(Debug, PartialEq, Default)]
pub struct SharedContext {
    pub completed: Vec<web_sys::Element>,
    pub events: Vec<()>,
    pub id: Option<usize>,
    pub registry: HashMap<String, web_sys::Element>,
}

impl SharedContext {
    pub fn new_with_registry(registry: HashMap<String, web_sys::Element>) -> Self {
        Self {
            completed: Default::default(),
            events: Default::default(),
            id: Some(0),
            registry,
        }
    }

    pub fn next_hydration_key(&mut self) -> usize {
        if let Some(id) = &mut self.id {
            let curr = *id;
            *id += 1;
            curr
        } else {
            self.id = Some(0);
            0
        }
    }

    pub fn gather(&self) {
        todo!()
    }
}
