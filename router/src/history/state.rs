use wasm_bindgen::JsValue;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct State(pub Option<JsValue>);

impl State {
    pub fn to_js_value(&self) -> JsValue {
        match &self.0 {
            Some(v) => v.clone(),
            None => JsValue::UNDEFINED,
        }
    }
}

impl<T> From<T> for State
where
    T: Into<JsValue>,
{
    fn from(value: T) -> Self {
        State(Some(value.into()))
    }
}
