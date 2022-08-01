use std::future::Future;

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
