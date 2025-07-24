use std::{
    ffi::c_void,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

pub type LoadCallbackFn = unsafe extern "C" fn(*const c_void, bool) -> ();
pub type LoadFn = unsafe extern "C" fn(LoadCallbackFn, *const c_void) -> ();

type Lazy = async_once_cell::Lazy<Option<()>, SplitLoaderFuture>;

use or_poisoned::OrPoisoned;
pub use wasm_split_macros::wasm_split;

pub struct LazySplitLoader {
    lazy: Pin<Arc<Lazy>>,
}

impl LazySplitLoader {
    pub fn new(load: LoadFn) -> Self {
        Self {
            lazy: Arc::pin(Lazy::new(SplitLoaderFuture::new(
                SplitLoader::new(load),
            ))),
        }
    }
}

pub async fn ensure_loaded(
    loader: &'static std::thread::LocalKey<LazySplitLoader>,
) -> Option<()> {
    *loader.with(|inner| inner.lazy.clone()).as_ref().await
}

#[derive(Clone, Copy, Debug)]
enum SplitLoaderState {
    Deferred(LoadFn),
    Pending,
    Completed(Option<()>),
}

struct SplitLoader {
    state: Mutex<SplitLoaderState>,
    waker: Mutex<Option<Waker>>,
}

impl SplitLoader {
    fn new(load: LoadFn) -> Arc<Self> {
        Arc::new(SplitLoader {
            state: Mutex::new(SplitLoaderState::Deferred(load)),
            waker: Mutex::new(None),
        })
    }

    fn complete(&self, value: bool) {
        *self.state.lock().or_poisoned() =
            SplitLoaderState::Completed(if value { Some(()) } else { None });
        if let Some(waker) = self.waker.lock().or_poisoned().take() {
            waker.wake();
        }
    }
}

struct SplitLoaderFuture {
    loader: Arc<SplitLoader>,
}

impl SplitLoaderFuture {
    fn new(loader: Arc<SplitLoader>) -> Self {
        SplitLoaderFuture { loader }
    }
}

impl Future for SplitLoaderFuture {
    type Output = Option<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<()>> {
        let mut loader = self.loader.state.lock().or_poisoned();
        match *loader {
            SplitLoaderState::Deferred(load) => {
                *loader = SplitLoaderState::Pending;
                *self.loader.waker.lock().or_poisoned() =
                    Some(cx.waker().clone());
                unsafe {
                    load(
                        load_callback,
                        Arc::<SplitLoader>::into_raw(self.loader.clone())
                            as *const c_void,
                    )
                };
                Poll::Pending
            }
            SplitLoaderState::Pending => {
                *self.loader.waker.lock().or_poisoned() =
                    Some(cx.waker().clone());
                Poll::Pending
            }
            SplitLoaderState::Completed(value) => Poll::Ready(value),
        }
    }
}

unsafe extern "C" fn load_callback(loader: *const c_void, success: bool) {
    unsafe { Arc::from_raw(loader as *const SplitLoader) }.complete(success);
}
