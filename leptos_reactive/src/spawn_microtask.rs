#![forbid(unsafe_code)]
use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(all(target_arch = "wasm32", any(feature = "csr", feature = "hydrate")))] {
        /// Exposes the [`queueMicrotask`](https://developer.mozilla.org/en-US/docs/Web/API/queueMicrotask) method
        /// in the browser, and simply runs the given function when on the server.
        pub fn queue_microtask(task: impl FnOnce() + 'static) {
            microtask(wasm_bindgen::closure::Closure::once_into_js(task));
        }

        #[wasm_bindgen::prelude::wasm_bindgen(
            inline_js = "export function microtask(f) { queueMicrotask(f); }"
        )]
        extern "C" {
            fn microtask(task: wasm_bindgen::JsValue);
        }
        // #[cfg(any(feature = "csr", feature = "hydrate"))]
        // fn microtask(task: wasm_bindgen::JsValue) {
        //     use js_sys::{Reflect, Function};
        //     use wasm_bindgen::prelude::*;
        //     let window = web_sys::window().expect("window not available");
        //     let queue_microtask = Reflect::get(&window, &JsValue::from_str("queueMicrotask")).expect("queueMicrotask not available");
        //     let queue_microtask = queue_microtask.unchecked_into::<Function>();
        //     let _ = queue_microtask.call0(&task);
        // }
    } else {
        /// Exposes the [`queueMicrotask`](https://developer.mozilla.org/en-US/docs/Web/API/queueMicrotask) method
        /// in the browser, and simply runs the given function when on the server.
        pub fn queue_microtask(task: impl FnOnce() + 'static) {
            task();
        }
    }
}
