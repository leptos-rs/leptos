use std::{any::Any, rc::Rc};

use leptos_dom::wasm_bindgen::JsValue;
use leptos_reactive::Memo;

use crate::ParamsMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub query: Memo<ParamsMap>,
    pub path_name: Memo<String>,
    pub search: Memo<String>,
    pub hash: Memo<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationChange {
    pub value: String,
    pub replace: bool,
    pub scroll: bool,
    pub state: State,
}

#[derive(Debug, Clone, Default)]
pub struct State(pub Option<Rc<dyn Any>>);

impl State {
    pub fn to_js_value(&self) -> JsValue {
        // TODO
        JsValue::UNDEFINED
    }
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        matches!((self.0.as_ref(), other.0.as_ref()), (None, None))
    }
}

impl Eq for State {}

impl Default for LocationChange {
    fn default() -> Self {
        Self {
            value: Default::default(),
            replace: true,
            scroll: true,
            state: Default::default(),
        }
    }
}
