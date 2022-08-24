use std::future::Future;

// run immediately on server
#[cfg(not(feature = "browser"))]
pub fn queue_microtask(task: impl FnOnce()) {
    task();
}

// run immediately on server
#[cfg(feature = "browser")]
pub fn queue_microtask(task: impl FnOnce() + 'static) {
    microtask(wasm_bindgen::closure::Closure::once_into_js(task));
}

#[cfg(feature = "browser")]
#[wasm_bindgen::prelude::wasm_bindgen(
    inline_js = "export function microtask(f) { queueMicrotask(f); }"
)]
extern "C" {
    fn microtask(task: wasm_bindgen::JsValue);
}

#[cfg(feature = "browser")]
pub fn spawn_local<F>(fut: F)
where
    F: Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(fut)
}

#[cfg(not(feature = "browser"))]
pub fn spawn_local<F>(_fut: F)
where
    F: Future<Output = ()> + 'static,
{
    // noop for now; useful for ignoring any async tasks on the server side
    // could be replaced with a Tokio dependency
}
