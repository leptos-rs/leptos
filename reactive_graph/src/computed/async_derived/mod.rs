mod arc_async_derived;
pub use arc_async_derived::*;
#[allow(clippy::module_inception)] // not a pub mod, who cares?
mod async_derived;
mod future_impls;
mod inner;
use crate::{
    graph::{AnySubscriber, Observer},
    owner::Owner,
};
pub use async_derived::*;
pub use future_impls::*;
use futures::Future;
use pin_project_lite::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

pin_project! {
    pub struct ScopedFuture<Fut> {
        owner: Option<Owner>,
        observer: Option<AnySubscriber>,
        #[pin]
        fut: Fut,
    }
}

impl<Fut> ScopedFuture<Fut> {
    pub fn new(fut: Fut) -> Self {
        let owner = Owner::current();
        let observer = Observer::get();
        Self {
            owner,
            observer,
            fut,
        }
    }
}

impl<Fut: Future> Future for ScopedFuture<Fut> {
    type Output = Fut::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match (this.owner, this.observer) {
            (None, None) => this.fut.poll(cx),
            (None, Some(obs)) => obs.with_observer(|| this.fut.poll(cx)),
            (Some(owner), None) => owner.with(|| this.fut.poll(cx)),
            (Some(owner), Some(observer)) => {
                owner.with(|| observer.with_observer(|| this.fut.poll(cx)))
            }
        }
    }
}

pub mod suspense {
    use crate::{
        signal::ArcRwSignal,
        traits::{Update, Writeable},
    };
    use futures::channel::oneshot::Sender;
    use or_poisoned::OrPoisoned;
    use slotmap::{DefaultKey, SlotMap};
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Debug)]
    pub struct LocalResourceNotifier(Arc<Mutex<Option<Sender<()>>>>);

    impl LocalResourceNotifier {
        pub fn notify(&mut self) {
            if let Some(tx) = self.0.lock().or_poisoned().take() {
                tx.send(()).unwrap();
            }
        }
    }

    impl From<Sender<()>> for LocalResourceNotifier {
        fn from(value: Sender<()>) -> Self {
            Self(Arc::new(Mutex::new(Some(value))))
        }
    }

    #[derive(Clone, Debug)]
    pub struct SuspenseContext {
        pub tasks: ArcRwSignal<SlotMap<DefaultKey, ()>>,
    }

    impl SuspenseContext {
        pub fn task_id(&self) -> TaskHandle {
            let key = self.tasks.write().insert(());
            TaskHandle {
                tasks: self.tasks.clone(),
                key,
            }
        }
    }

    /// A unique identifier that removes itself from the set of tasks when it is dropped.
    #[derive(Debug)]
    pub struct TaskHandle {
        tasks: ArcRwSignal<SlotMap<DefaultKey, ()>>,
        key: DefaultKey,
    }

    impl Drop for TaskHandle {
        fn drop(&mut self) {
            self.tasks.update(|tasks| {
                tasks.remove(self.key);
            });
        }
    }
}
