use futures::{channel::oneshot, future::join_all};
use or_poisoned::OrPoisoned;
use std::{
    future::Future,
    sync::{mpsc, OnceLock, RwLock},
};

static TRANSITION: OnceLock<RwLock<Option<TransitionInner>>> = OnceLock::new();

fn global_transition() -> &'static RwLock<Option<TransitionInner>> {
    TRANSITION.get_or_init(|| RwLock::new(None))
}

#[derive(Debug, Clone)]
struct TransitionInner {
    tx: mpsc::Sender<oneshot::Receiver<()>>,
}

#[derive(Debug)]
pub struct AsyncTransition;

impl AsyncTransition {
    pub async fn run<T, U>(action: impl FnOnce() -> T) -> U
    where
        T: Future<Output = U>,
    {
        let (tx, rx) = mpsc::channel();
        let global_transition = global_transition();
        let inner = TransitionInner { tx };
        let prev = std::mem::replace(
            &mut *global_transition.write().or_poisoned(),
            Some(inner.clone()),
        );
        let value = action().await;
        _ = std::mem::replace(
            &mut *global_transition.write().or_poisoned(),
            prev,
        );
        let mut pending = Vec::new();
        while let Ok(tx) = rx.try_recv() {
            pending.push(tx);
        }
        join_all(pending).await;
        value
    }

    pub(crate) fn register(rx: oneshot::Receiver<()>) {
        if let Some(tx) = global_transition()
            .read()
            .or_poisoned()
            .as_ref()
            .map(|n| &n.tx)
        {
            // if it's an Err, that just means the Receiver was dropped
            // i.e., the transition is no longer listening, in which case it doesn't matter if we
            // successfully register with it or not
            _ = tx.send(rx);
        }
    }
}
