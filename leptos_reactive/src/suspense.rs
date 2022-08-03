use crate::{ReadSignal, Scope, WriteSignal};

#[derive(Clone)]
pub struct SuspenseContext {
    pending_resources: ReadSignal<usize>,
    set_pending_resources: WriteSignal<usize>,
}

impl SuspenseContext {
    pub fn new(cx: Scope) -> Self {
        let (pending_resources, set_pending_resources) = cx.create_signal_owned(0);
        Self {
            pending_resources,
            set_pending_resources,
        }
    }

    pub fn increment(&self) {
        self.set_pending_resources.update(|n| *n += 1);
    }

    pub fn decrement(&self) {
        self.set_pending_resources.update(|n| *n -= 1);
    }

    pub fn ready(&self) -> bool {
        *self.pending_resources.get() == 0
    }
}
