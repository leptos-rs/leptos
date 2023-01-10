// `queue_microtask` needs to be in its own module, which is the only thing
// in this entire framework that requires "unsafe" code (because Rust seems to
// that a `wasm_bindgen` imported function like this is unsafe)
// this is stupid, and one day hopefully web_sys will add queue_microtask itself

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(any(feature = "csr", feature = "hydrate"))] {
        /// Exposes the [queueMicrotask](https://developer.mozilla.org/en-US/docs/Web/API/queueMicrotask) method
        /// in the browser, and simply runs the given function when on the server.
        pub fn queue_microtask(task: impl FnOnce() + 'static) {
            microtask(wasm_bindgen::closure::Closure::once_into_js(task));
        }

        #[cfg(any(feature = "csr", feature = "hydrate"))]
        #[wasm_bindgen::prelude::wasm_bindgen(
            inline_js = "export function microtask(f) { queueMicrotask(f); }"
        )]
        extern "C" {
            fn microtask(task: wasm_bindgen::JsValue);
        }
    } else {
        /// Exposes the [queueMicrotask](https://developer.mozilla.org/en-US/docs/Web/API/queueMicrotask) method
        /// in the browser, and simply runs the given function when on the server.
        #[cfg(not(any(feature = "csr", feature = "hydrate")))]
        pub fn queue_microtask(task: impl FnOnce()) {
            task();
        }
    }
}
