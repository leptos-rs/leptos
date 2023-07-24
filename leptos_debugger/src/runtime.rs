use crate::Message;
use std::cell::RefCell;

thread_local! {
    pub(crate) static RUNTIME: Runtime = Default::default();
}

#[derive(Default)]
pub(crate) struct Runtime {
    pub hook: RefCell<Option<Box<dyn Fn(Message)>>>,
}

pub(crate) fn with_runtime<T>(f: impl FnOnce(&Runtime) -> T) -> T {
    RUNTIME.with(|runtime| f(runtime))
}
