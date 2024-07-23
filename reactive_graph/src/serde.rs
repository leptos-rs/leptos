use crate::{
    computed::{ArcMemo, Memo},
    owner::Storage,
    signal::{ArcReadSignal, ArcRwSignal, ReadSignal, RwSignal},
    traits::With,
    wrappers::read::{MaybeProp, MaybeSignal, Signal, SignalTypes},
};
use serde::{Deserialize, Serialize};

impl<T, St> Serialize for ReadSignal<T, St>
where
    T: Serialize + 'static,
    St: Storage<ArcReadSignal<T>>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.with(|value| value.serialize(serializer))
    }
}

impl<T, St> Serialize for RwSignal<T, St>
where
    T: Serialize + 'static,
    St: Storage<ArcRwSignal<T>>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.with(|value| value.serialize(serializer))
    }
}

impl<T, St> Serialize for Memo<T, St>
where
    T: Send + Sync + Serialize + 'static,
    St: Storage<ArcMemo<T>>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.with(|value| value.serialize(serializer))
    }
}

impl<T: Serialize + 'static> Serialize for ArcReadSignal<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.with(|value| value.serialize(serializer))
    }
}

impl<T: Serialize + 'static> Serialize for ArcRwSignal<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.with(|value| value.serialize(serializer))
    }
}

impl<T: Send + Sync + Serialize + 'static> Serialize for ArcMemo<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.with(|value| value.serialize(serializer))
    }
}

impl<T, St> Serialize for MaybeSignal<T, St>
where
    T: Send + Sync + Serialize,
    St: Storage<SignalTypes<T>>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.with(|value| value.serialize(serializer))
    }
}

impl<T, St> Serialize for MaybeProp<T, St>
where
    T: Send + Sync + Serialize,
    St: Storage<SignalTypes<Option<T>>>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match &self.0 {
            None | Some(MaybeSignal::Static(None)) => {
                None::<T>.serialize(serializer)
            }
            Some(MaybeSignal::Static(Some(value))) => {
                value.serialize(serializer)
            }
            Some(MaybeSignal::Dynamic(signal)) => {
                signal.with(|value| value.serialize(serializer))
            }
        }
    }
}

impl<T, St> Serialize for Signal<T, St>
where
    T: Send + Sync + Serialize + 'static,
    St: Storage<SignalTypes<T>>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.with(|value| value.serialize(serializer))
    }
}

/* Deserialization for signal types */

impl<'de, T, S> Deserialize<'de> for RwSignal<T, S>
where
    T: Send + Sync + Deserialize<'de> + 'static,
    S: Storage<ArcRwSignal<T>>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(RwSignal::new_with_storage)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for ArcRwSignal<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(ArcRwSignal::new)
    }
}

impl<'de, T: Deserialize<'de>, S> Deserialize<'de> for MaybeSignal<T, S> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(MaybeSignal::Static)
    }
}
