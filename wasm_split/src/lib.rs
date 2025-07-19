use std::{
    cell::Cell,
    ffi::c_void,
    future::Future,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

pub type LoadCallbackFn = unsafe extern "C" fn(*const c_void, bool) -> ();
pub type LoadFn = unsafe extern "C" fn(LoadCallbackFn, *const c_void) -> ();

type Lazy = async_once_cell::Lazy<Option<()>, SplitLoaderFuture>;

pub use wasm_split_macros::wasm_split;

pub struct LazySplitLoader {
    lazy: Pin<Rc<Lazy>>,
}

impl LazySplitLoader {
    pub fn new(load: LoadFn) -> Self {
        Self {
            lazy: Rc::pin(Lazy::new(SplitLoaderFuture::new(SplitLoader::new(
                load,
            )))),
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
    state: Cell<SplitLoaderState>,
    waker: Cell<Option<Waker>>,
}

impl SplitLoader {
    fn new(load: LoadFn) -> Rc<Self> {
        Rc::new(SplitLoader {
            state: Cell::new(SplitLoaderState::Deferred(load)),
            waker: Cell::new(None),
        })
    }

    fn complete(&self, value: bool) {
        self.state.set(SplitLoaderState::Completed(if value {
            Some(())
        } else {
            None
        }));
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
    }
}

struct SplitLoaderFuture {
    loader: Rc<SplitLoader>,
}

impl SplitLoaderFuture {
    fn new(loader: Rc<SplitLoader>) -> Self {
        SplitLoaderFuture { loader }
    }
}

impl Future for SplitLoaderFuture {
    type Output = Option<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<()>> {
        match self.loader.state.get() {
            SplitLoaderState::Deferred(load) => {
                self.loader.state.set(SplitLoaderState::Pending);
                self.loader.waker.set(Some(cx.waker().clone()));
                unsafe {
                    load(
                        load_callback,
                        Rc::<SplitLoader>::into_raw(self.loader.clone())
                            as *const c_void,
                    )
                };
                Poll::Pending
            }
            SplitLoaderState::Pending => {
                self.loader.waker.set(Some(cx.waker().clone()));
                Poll::Pending
            }
            SplitLoaderState::Completed(value) => Poll::Ready(value),
        }
    }
}

unsafe extern "C" fn load_callback(loader: *const c_void, success: bool) {
    unsafe { Rc::from_raw(loader as *const SplitLoader) }.complete(success);
}
