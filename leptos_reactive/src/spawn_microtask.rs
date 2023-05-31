// `queue_microtask` needs to be in its own module, which is the only thing
// in this entire framework that requires "unsafe" code (because Rust seems to
// that a `wasm_bindgen` imported function like this is unsafe)
// this is stupid, and one day hopefully web_sys will add queue_microtask itself

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(all(target_arch = "wasm32", any(feature = "csr", feature = "hydrate")))] {
        /// Exposes the [`queueMicrotask`](https://developer.mozilla.org/en-US/docs/Web/API/queueMicrotask) method
        /// in the browser, and simply runs the given function when on the server.
        pub fn queue_microtask(task: impl FnOnce() + 'static) {
            microtask(wasm_bindgen::closure::Closure::once_into_js(task));
        }

        #[cfg(any(feature = "csr", feature = "hydrate"))]
        fn microtask(task: wasm_bindgen::JsValue) {
            use js_sys::{Reflect, Function};
            use wasm_bindgen::prelude::*;
            let window = web_sys::window().expect("window not available");
            let queue_microtask = Reflect::get(&window, &JsValue::from_str("queueMicrotask")).expect("queueMicrotask not available");
            let queue_microtask = queue_microtask.dyn_into::<Function>().expect("queueMicrotask not a function");
            let _ = queue_microtask.call0(&task);
        }
    } else {
        /// Exposes the [`queueMicrotask`](https://developer.mozilla.org/en-US/docs/Web/API/queueMicrotask) method
        /// in the browser, and simply runs the given function when on the server.
        pub fn queue_microtask(task: impl FnOnce() + 'static) {
            task();
        }
    }
}
