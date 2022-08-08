use std::collections::HashMap;

#[derive(Clone, Debug)]
pub(crate) struct ScopeArena<T> {
    next_idx: usize,
    items: HashMap<Index, T>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ScopeId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Index {
    index: usize,
}

impl<T> Default for ScopeArena<T> {
    fn default() -> ScopeArena<T> {
        ScopeArena::new()
    }
}

impl<T> ScopeArena<T> {
    pub(crate) fn new() -> ScopeArena<T> {
        ScopeArena {
            next_idx: 0,
            items: HashMap::new(),
        }
    }

    pub(crate) fn insert(&mut self, value: T) -> Index {
        let index = Index {
            index: self.next_idx,
        };
        self.next_idx += 1;
        self.items.insert(index, value);
        index
    }

    pub(crate) fn remove(&mut self, idx: Index) -> Option<T> {
        self.items.remove(&idx)
    }

    pub(crate) fn drain(&mut self) -> impl Iterator<Item = (Index, T)> + '_ {
        self.items.drain()
    }
}
