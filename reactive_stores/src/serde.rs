use crate::Store;
use reactive_graph::{
    owner::{LocalStorage, SyncStorage},
    traits::With,
};
use serde::{Deserialize, Serialize, Serializer};

impl<T: Serialize> Serialize for Store<T>
where
    Store<T>: With<Value = T>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.with(|item| item.serialize(serializer))
    }
}

impl<'de, T: Deserialize<'de> + Send + Sync + 'static> Deserialize<'de>
    for Store<T, SyncStorage>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(|inner| Store::new(inner))
    }
}

impl<'de, T: Deserialize<'de> + 'static> Deserialize<'de>
    for Store<T, LocalStorage>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(|inner| Store::new_local(inner))
    }
}
