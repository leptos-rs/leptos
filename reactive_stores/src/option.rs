use crate::{StoreField, Subfield};
use reactive_graph::traits::Read;
use std::ops::Deref;

pub trait OptionStoreExt
where
    Self: StoreField<Value = Option<Self::Output>>,
{
    type Output;

    fn unwrap(self) -> Subfield<Self, Option<Self::Output>, Self::Output>;

    fn map<U>(
        self,
        map_fn: impl FnOnce(Subfield<Self, Option<Self::Output>, Self::Output>) -> U,
    ) -> Option<U>;
}

impl<T, S> OptionStoreExt for S
where
    S: StoreField<Value = Option<T>> + Read,
    <S as Read>::Value: Deref<Target = Option<T>>,
{
    type Output = T;

    fn unwrap(self) -> Subfield<Self, Option<Self::Output>, Self::Output> {
        Subfield::new(
            self,
            0.into(),
            |t| t.as_ref().unwrap(),
            |t| t.as_mut().unwrap(),
        )
    }

    fn map<U>(
        self,
        map_fn: impl FnOnce(Subfield<S, Option<T>, T>) -> U,
    ) -> Option<U> {
        if self.read().is_some() {
            Some(map_fn(self.unwrap()))
        } else {
            None
        }
    }
}
