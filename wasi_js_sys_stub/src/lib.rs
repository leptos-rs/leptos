pub mod Reflect {
    use wasm_bindgen::JsValue;
    pub fn set(
        target: &JsValue,
        property: &JsValue,
        value: &JsValue,
    ) -> Result<bool, JsValue> {
        let _ = (target, property, value);
        Ok(true)
    }
}
