use crate::{
    ArcField, ArcStore, AtIndex, AtKeyed, DerefedField, Field, KeyedIterable,
    KeyedSubfield, Store, StoreField, Subfield,
};
use reactive_graph::{
    owner::Storage,
    traits::{Get, SignalOrFn},
};
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut, Index},
};

impl<Inner, Prev, V> SignalOrFn for Subfield<Inner, Prev, V>
where
    Subfield<Inner, Prev, V>: Get<Value = V>,
    Prev: Send + Sync + 'static,
    Inner: Send + Sync + Clone + 'static,
{
    type Output = V;

    fn run(&self) -> V {
        Get::get(self)
    }
}

impl<Inner, Prev, K, V> SignalOrFn for AtKeyed<Inner, Prev, K, V>
where
    AtKeyed<Inner, Prev, K, V>: Get<Value = V>,
    Prev: Send + Sync + 'static,
    Inner: Send + Sync + Clone + 'static,
    K: Send + Sync + Debug + Clone + 'static,
    V: KeyedIterable,
{
    type Output = V;

    fn run(&self) -> V {
        Get::get(self)
    }
}

impl<Inner, Prev, K, V> SignalOrFn for KeyedSubfield<Inner, Prev, K, V>
where
    KeyedSubfield<Inner, Prev, K, V>: Get<Value = V>,
    Prev: Send + Sync + 'static,
    Inner: Send + Sync + Clone + 'static,
    K: Send + Sync + Debug + Clone + 'static,
    V: KeyedIterable,
{
    type Output = V;

    fn run(&self) -> V {
        Get::get(self)
    }
}

impl<S> SignalOrFn for DerefedField<S>
where
    S: Clone + StoreField + Send + Sync + 'static,
    <S as StoreField>::Value: Deref + DerefMut,
    <<S as StoreField>::Value as Deref>::Target: Sized,
    DerefedField<S>: Get<Value = <<S as StoreField>::Value as Deref>::Target>,
{
    type Output = <<S as StoreField>::Value as Deref>::Target;

    fn run(&self) -> Self::Output {
        Get::get(self)
    }
}

impl<Inner, Prev> SignalOrFn for AtIndex<Inner, Prev>
where
    AtIndex<Inner, Prev>: Get<Value = <Prev as Index<usize>>::Output>,
    Prev: Send + Sync + Index<usize> + 'static,
    <Prev as Index<usize>>::Output: Sized,
    Inner: Send + Sync + Clone + 'static,
{
    type Output = <Prev as Index<usize>>::Output;

    fn run(&self) -> Self::Output {
        Get::get(self)
    }
}

impl<V, S> SignalOrFn for Store<V, S>
where
    Store<V, S>: Get<Value = V>,
    S: Storage<V> + Storage<Option<V>> + Send + Sync + 'static,
{
    type Output = V;

    fn run(&self) -> V {
        Get::get(self)
    }
}

impl<V, S> SignalOrFn for Field<V, S>
where
    Field<V, S>: Get<Value = V>,
    S: Storage<V> + Storage<Option<V>> + Send + Sync + 'static,
{
    type Output = V;

    fn run(&self) -> V {
        Get::get(self)
    }
}

impl<V> SignalOrFn for ArcStore<V>
where
    ArcStore<V>: Get<Value = V>,
{
    type Output = V;

    fn run(&self) -> V {
        Get::get(self)
    }
}

impl<V> SignalOrFn for ArcField<V>
where
    ArcField<V>: Get<Value = V>,
{
    type Output = V;

    fn run(&self) -> V {
        Get::get(self)
    }
}
