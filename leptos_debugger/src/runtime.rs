use crate::{DNode, Hook, Prop, PropValue};
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    pub(crate) static RUNTIME: Runtime = Default::default();
    pub(crate) static HOOK: Option<Box<dyn Hook>> = Default::default();
    pub(crate) static CONFIG: Config = Default::default();
}

#[derive(Default)]
pub(crate) struct Config {
    pub is_root: bool,
}

#[derive(Default)]
pub(crate) struct Runtime {
    pub nodes: RefCell<HashMap<String, Vec<DNode>>>,
    pub props: RefCell<HashMap<String, HashMap<String, Prop>>>,
    pub signals: RefCell<HashMap<u64, PropValue>>,

    pub hook: RefCell<Option<Box<dyn Hook>>>,
    pub config: RefCell<Config>,
}

pub(crate) fn with_runtime<T>(f: impl FnOnce(&Runtime) -> T) -> T {
    RUNTIME.with(|runtime| f(runtime))
}
