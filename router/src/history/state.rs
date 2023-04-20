use wasm_bindgen::JsValue;

pub const FORWARD: &str = "forward";
pub const STATE: &str = "state";

#[derive(Debug, Clone, Default, PartialEq)]
pub struct State(pub Option<JsValue>);

impl State {
    pub fn to_js_value(&self) -> JsValue {
        match &self.0 {
            Some(v) => v.clone(),
            None => js_sys::Object::new().into(),
        }
    }

    pub fn to_object(&self, forward: bool) -> JsValue {
        let obj = js_sys::Object::new();
        _ = js_sys::Reflect::set(&obj, &JsValue::from_str(FORWARD), &JsValue::from_bool(forward));
        _ = js_sys::Reflect::set(&obj, &JsValue::from_str(STATE), &self.to_js_value());
        obj.into()
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
