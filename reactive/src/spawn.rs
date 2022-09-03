use std::future::Future;

// run immediately on server
#[cfg(feature = "ssr")]
pub fn queue_microtask(task: impl FnOnce()) {
    task();
}

// run immediately on server
#[cfg(any(feature = "csr", feature = "hydrate"))]
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

#[cfg(any(feature = "csr", feature = "hydrate"))]
pub fn spawn_local<F>(fut: F)
where
    F: Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(fut)
}

#[cfg(feature = "ssr")]
pub fn spawn_local<F>(_fut: F)
where
    F: Future<Output = ()> + 'static,
{
    // noop for now; useful for ignoring any async tasks on the server side
    // could be replaced with a Tokio dependency
}
