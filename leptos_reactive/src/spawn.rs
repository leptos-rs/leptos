use std::future::Future;

// run immediately on server
#[cfg(not(target_arch = "wasm32"))]
pub fn queue_microtask(task: impl FnOnce()) {
    task();
}

// run immediately on server
#[cfg(target_arch = "wasm32")]
pub fn queue_microtask(task: impl FnOnce() + 'static) {
    microtask(wasm_bindgen::closure::Closure::once_into_js(task));
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(
    inline_js = "export function microtask(f) { queueMicrotask(f); }"
)]
extern "C" {
    fn microtask(task: wasm_bindgen::JsValue);
}

#[cfg(target_arch = "wasm32")]
pub fn spawn_local<F>(fut: F)
where
    F: Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(fut)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_local<F>(_fut: F)
where
    F: Future<Output = ()> + 'static,
{

    // noop for now; useful for ignoring any async tasks on the server side
    // could be replaced with a Tokio dependency
}
