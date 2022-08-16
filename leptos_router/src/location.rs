use std::{any::Any, rc::Rc};

use leptos_reactive::Memo;

use crate::Params;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub query: Memo<Params>,
    pub path_name: Memo<String>,
    pub search: Memo<String>,
    pub hash: Memo<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationChange {
    pub value: String,
    pub replace: bool,
    pub scroll: bool,
    pub state: State,
}

#[derive(Debug, Clone)]
pub struct State(pub Option<Rc<dyn Any>>);

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        matches!((self.0.as_ref(), other.0.as_ref()), (None, None))
    }
}

impl Eq for State {}

/* pub trait State {}

impl<T> State for T where T: Any + std::fmt::Debug + PartialEq + Eq + Clone {}
 */
