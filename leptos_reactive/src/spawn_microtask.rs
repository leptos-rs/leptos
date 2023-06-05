#![forbid(unsafe_code)]

pub fn queue_microtask(task: impl FnOnce() + 'static) {
    #[cfg(not(any(feature = "hydrate", feature = "csr")))]
    {
        task();
    }

    #[cfg(any(feature = "hydrate", feature = "csr"))]
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
