use core::sync::atomic::Ordering::Relaxed;
use futures::{task::AtomicWaker, Stream};
use std::{
    fmt::Debug,
    hash::Hash,
    pin::Pin,
    sync::{atomic::AtomicBool, Arc, Weak},
    task::{Context, Poll},
};

#[derive(Debug)]
pub(crate) struct Sender(Arc<Inner>);

#[derive(Debug)]
pub(crate) struct Receiver(Weak<Inner>);

#[derive(Debug, Default)]
struct Inner {
    waker: AtomicWaker,
    set: AtomicBool,
}

impl Drop for Inner {
    fn drop(&mut self) {
        // Sender holds a strong reference to Inner, and Receiver holds a weak reference to Inner,
        // so this will run when the Sender is dropped.
        //
        // The Receiver is usually owned by a spawned async task that is always waiting on the next
        // value from its Stream. While it's waiting, it continues owning all the data it has
        // captured. That data will not be dropped until the the stream ends.
        //
        // If we don't wake the waker a final time here, the spawned task will continue waiting for
        // a final message from the Receiver that never arrives, because the waker never wakes it
        // up again. So we wake the waker a final time, which tries to upgrade the Receiver, which
        // fails, which causes the stream to yield Poll::Ready(None), ending the stream, and
        // therefore ending the task, and therefore dropping all data that the stream has
        // captured, avoiding a memory leak.
        self.waker.wake();
    }
}

pub fn channel() -> (Sender, Receiver) {
    let inner = Arc::new(Inner {
        waker: AtomicWaker::new(),
        set: AtomicBool::new(false),
    });
    let rx = Arc::downgrade(&inner);
    (Sender(inner), Receiver(rx))
}

impl Sender {
    pub fn notify(&mut self) {
        self.0.set.store(true, Relaxed);
        self.0.waker.wake();
    }
}

impl Stream for Receiver {
    type Item = ();

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        if let Some(inner) = self.0.upgrade() {
            inner.waker.register(cx.waker());

            if inner.set.swap(false, Relaxed) {
                Poll::Ready(Some(()))
            } else {
                Poll::Pending
            }
        } else {
            Poll::Ready(None)
        }
    }
}

impl Hash for Sender {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).hash(state)
    }
}

impl PartialEq for Sender {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Sender {}

impl Hash for Receiver {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Weak::as_ptr(&self.0).hash(state)
    }
}

impl PartialEq for Receiver {
    fn eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Receiver {}
