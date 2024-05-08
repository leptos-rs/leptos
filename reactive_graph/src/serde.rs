use crate::{
    computed::{ArcMemo, Memo},
    signal::{ArcReadSignal, ArcRwSignal, ReadSignal, RwSignal},
    traits::With,
    wrappers::read::{MaybeSignal, Signal},
};
use serde::{Deserialize, Serialize};

impl<T: Send + Sync + Serialize + 'static> Serialize for ReadSignal<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.with(|value| value.serialize(serializer))
    }
}

impl<T: Send + Sync + Serialize + 'static> Serialize for RwSignal<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.with(|value| value.serialize(serializer))
    }
}

impl<T: Send + Sync + Serialize + 'static> Serialize for Memo<T> {
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

impl<T: Send + Sync + Serialize> Serialize for MaybeSignal<T> {
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

impl<T: Send + Sync + Clone + Serialize> Serialize for Signal<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.with(|value| value.serialize(serializer))
    }
}

/* Deserialization for signal types */

impl<'de, T: Send + Sync + Deserialize<'de>> Deserialize<'de> for RwSignal<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(RwSignal::new)
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

impl<'de, T: Deserialize<'de>> Deserialize<'de> for MaybeSignal<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(MaybeSignal::Static)
    }
}
