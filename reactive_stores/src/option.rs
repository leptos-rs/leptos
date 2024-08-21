use crate::{StoreField, Subfield};

pub trait OptionStoreExt
where
    Self: StoreField<Value = Option<Self::Output>>,
{
    type Output;

    fn unwrap(self) -> Subfield<Self, Option<Self::Output>, Self::Output>;
}

impl<T, S> OptionStoreExt for S
where
    S: StoreField<Value = Option<T>>,
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
}
