use crate::{
    create_rw_signal, MaybeProp, MaybeSignal, Memo, ReadSignal, RwSignal,
    Signal, SignalGet, SignalWith,
};
use serde::{Deserialize, Serialize};

/* Serialization for signal types */

impl<T: Serialize> Serialize for ReadSignal<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.with(|value| value.serialize(serializer))
    }
}

impl<T: Serialize> Serialize for RwSignal<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.with(|value| value.serialize(serializer))
    }
}

impl<T: Serialize> Serialize for Memo<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.with(|value| value.serialize(serializer))
    }
}

impl<T: Serialize> Serialize for MaybeSignal<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.with(|value| value.serialize(serializer))
    }
}

impl<T: Serialize> Serialize for MaybeProp<T> {
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

impl<T: Clone + Serialize> Serialize for Signal<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.get().serialize(serializer)
    }
}

/* Deserialization for signal types */

impl<'de, T: Deserialize<'de>> Deserialize<'de> for RwSignal<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(create_rw_signal)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for MaybeSignal<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(MaybeSignal::Static)
    }
}
