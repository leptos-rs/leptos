use cfg_if::cfg_if;
use std::future::Future;

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

/// Spawns and runs a thread-local [std::future::Future] in a platform-independent way.
///
/// This can be used to interface with any `async` code.
pub fn spawn_local<F>(fut: F)
where
    F: Future<Output = ()> + 'static,
{
    cfg_if::cfg_if! {
        if #[cfg(any(feature = "csr", feature = "hydrate"))] {
            wasm_bindgen_futures::spawn_local(fut)
        }
        else if #[cfg(any(test, doctest))] {
            tokio_test::block_on(fut);
        } else if #[cfg(feature = "ssr")] {
            use tokio::task;
            let local = task::LocalSet::new();

            local.run_until(async move {
                tokio::task::spawn_local(fut);
            });

        }  else {
            futures::executor::block_on(fut)
        }
    }
}
