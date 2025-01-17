#[cfg(not(feature = "ssr"))]
mod csr {
    use gloo_utils::format::JsValueSerdeExt;
    use js_sys::{
        Object,
        Reflect::{get, set},
    };
    use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

    #[wasm_bindgen(
        module = "/node_modules/@highlightjs/cdn-assets/es/highlight.min.js"
    )]
    extern "C" {
        type HighlightOptions;

        #[wasm_bindgen(catch, js_namespace = defaultMod, js_name = highlight)]
        fn highlight_lang(
            code: String,
            options: Object,
        ) -> Result<Object, JsValue>;

        #[wasm_bindgen(js_namespace = defaultMod, js_name = highlightAll)]
        pub fn highlight_all();
    }

    // Keeping the `ignoreIllegals` argument out of the default case, and since there is no optional arguments
    // in Rust, this will have to be provided in a separate function (e.g. `highlight_ignore_illegals`), much
    // like how `web_sys` does it for the browser APIs.  For simplicity, only the highlighted HTML code is
    // returned on success, and None on error.
    pub fn highlight(code: String, lang: String) -> Option<String> {
        let options = js_sys::Object::new();
        set(&options, &"language".into(), &lang.into())
            .expect("failed to assign lang to options");
        highlight_lang(code, options)
            .map(|result| {
                let value = get(&result, &"value".into())
                    .expect("HighlightResult failed to contain the value key");
                value.into_serde().expect("Value should have been a string")
            })
            .ok()
    }
}

#[cfg(feature = "ssr")]
mod ssr {
    // noop under ssr
    pub fn highlight_all() {}

    // TODO see if there is a Rust-based solution that will enable isomorphic rendering for this feature.
    // the current (disabled) implementation simply calls html_escape.
    // pub fn highlight(code: String, _lang: String) -> Option<String> {
    //     Some(html_escape::encode_text(&code).into_owned())
    // }
}

#[cfg(not(feature = "ssr"))]
pub use csr::*;
#[cfg(feature = "ssr")]
pub use ssr::*;
