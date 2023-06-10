#![forbid(unsafe_code)]

/// The microtask is a short function which will run after the current task has
/// completed its work and when there is no other code waiting to be run before
/// control of the execution context is returned to the browser's event loop.
///
/// Microtasks are especially useful for libraries and frameworks that need
/// to perform final cleanup or other just-before-rendering tasks.
///
/// [MDN queueMicrotask](https://developer.mozilla.org/en-US/docs/Web/API/queueMicrotask)
pub fn queue_microtask(task: impl FnOnce() + 'static) {
    #[cfg(not(all(
        target_arch = "wasm32",
        any(feature = "hydrate", feature = "csr")
    )))]
    {
        task();
    }

    #[cfg(all(
        target_arch = "wasm32",
        any(feature = "hydrate", feature = "csr")
    ))]
    {
        use js_sys::{Function, Reflect};
        use wasm_bindgen::prelude::*;

        let task = Closure::once_into_js(task);
        let window = web_sys::window().expect("window not available");
        let queue_microtask =
            Reflect::get(&window, &JsValue::from_str("queueMicrotask"))
                .expect("queueMicrotask not available");
        let queue_microtask = queue_microtask.unchecked_into::<Function>();
        _ = queue_microtask.call1(&JsValue::UNDEFINED, &task);
    }
}
