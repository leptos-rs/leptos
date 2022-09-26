#[cfg(not(feature = "ssr"))]
use wasm_bindgen::JsValue;

#[derive(Debug, Clone, Default, PartialEq)]
#[cfg(not(feature = "ssr"))]
pub struct State(pub Option<JsValue>);

#[derive(Debug, Clone, Default, PartialEq)]
#[cfg(feature = "ssr")]
pub struct State(pub Option<()>);

impl State {
    #[cfg(not(feature = "ssr"))]
    pub fn to_js_value(&self) -> JsValue {
        match &self.0 {
            Some(v) => v.clone(),
            None => JsValue::UNDEFINED,
        }
    }
}

#[cfg(not(feature = "ssr"))]
impl<T> From<T> for State
where
    T: Into<JsValue>,
{
    fn from(value: T) -> Self {
        State(Some(value.into()))
    }
}
