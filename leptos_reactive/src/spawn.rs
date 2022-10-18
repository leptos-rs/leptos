use std::future::Future;

/// Exposes the [queueMicrotask](https://developer.mozilla.org/en-US/docs/Web/API/queueMicrotask) method
/// in the browser, and simply runs the given function when on the server.
#[cfg(not(target_arch = "wasm32"))]
pub fn queue_microtask(task: impl FnOnce()) {
    task();
}

/// Exposes the [queueMicrotask](https://developer.mozilla.org/en-US/docs/Web/API/queueMicrotask) method
/// in the browser, and simply runs the given function when on the server.
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

pub fn spawn_local<F>(fut: F)
where
    F: Future<Output = ()> + 'static,
{
    cfg_if::cfg_if! {
        if #[cfg(any(feature = "csr", feature = "hydrate"))] {
            wasm_bindgen_futures::spawn_local(fut)
        } else if #[cfg(feature = "ssr")] {
            tokio::task::spawn_local(fut);
        } else if #[cfg(any(test, doctest))] {
            tokio_test::block_on(fut);
        } else {
            futures::executor::block_on(fut)
        }
    }
}
