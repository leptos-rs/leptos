#[allow(non_snake_case)]
pub mod Reflect {
    use crate::wasm_bindgen::JsValue;
    pub fn set(
        target: &JsValue,
        property: &JsValue,
        value: &JsValue,
    ) -> Result<bool, JsValue> {
        let _ = (target, property, value);
        Ok(true)
    }
}
