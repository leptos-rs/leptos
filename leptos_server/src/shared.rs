#[cfg(feature = "miniserde")]
use crate::serializers::Miniserde;
#[cfg(feature = "rkyv")]
use crate::serializers::Rkyv;
#[cfg(feature = "serde-lite")]
use crate::serializers::SerdeLite;
#[cfg(feature = "serde-wasm-bindgen")]
use crate::serializers::SerdeWasmBindgen;
use crate::serializers::{SerdeJson, SerializableData, Serializer, Str};
use std::{
    fmt::{Debug, Display},
    hash::Hash,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

/// A smart pointer that allows you to share identical, synchronously-loaded data between the
/// server and the client.
///
/// If this constructed on the server, it serializes its value into the shared context. If it is
/// constructed on the client during hydration, it reads its value from the shared context. If
/// it it constructed on the client at any other time, it simply runs on the client.
#[derive(Debug)]
pub struct SharedValue<T, Ser = SerdeJson> {
    value: T,
    ser: PhantomData<Ser>,
}

impl<T, Ser> SharedValue<T, Ser> {
    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T> SharedValue<T, SerdeJson>
where
    T: Debug + SerializableData<SerdeJson>,
    T::SerErr: Debug,
    T::DeErr: Debug,
{
    pub fn new(initial: impl FnOnce() -> T) -> Self {
        SharedValue::new_with_encoding(initial)
    }
}

impl<T> SharedValue<T, Str>
where
    T: Debug + SerializableData<Str>,
    T::SerErr: Debug,
    T::DeErr: Debug,
{
    pub fn new_str(initial: impl FnOnce() -> T) -> Self {
        SharedValue::new_with_encoding(initial)
    }
}

#[cfg(feature = "serde-lite")]
impl<T> SharedValue<T, SerdeLite>
where
    T: Debug + SerializableData<SerdeLite>,
    T::SerErr: Debug,
    T::DeErr: Debug,
{
    pub fn new(initial: impl FnOnce() -> T) -> Self {
        SharedValue::new_with_encoding(initial)
    }
}

#[cfg(feature = "serde-wasm-bindgen")]
impl<T> SharedValue<T, SerdeWasmBindgen>
where
    T: Debug + SerializableData<SerdeWasmBindgen>,
    T::SerErr: Debug,
    T::DeErr: Debug,
{
    pub fn new(initial: impl FnOnce() -> T) -> Self {
        SharedValue::new_with_encoding(initial)
    }
}

#[cfg(feature = "miniserde")]
impl<T> SharedValue<T, Miniserde>
where
    T: Debug + SerializableData<Miniserde>,
    T::SerErr: Debug,
    T::DeErr: Debug,
{
    pub fn new(initial: impl FnOnce() -> T) -> Self {
        SharedValue::new_with_encoding(initial)
    }
}

#[cfg(feature = "rkyv")]
impl<T> SharedValue<T, Rkyv>
where
    T: Debug + SerializableData<Rkyv>,
    T::SerErr: Debug,
    T::DeErr: Debug,
{
    pub fn new(initial: impl FnOnce() -> T) -> Self {
        SharedValue::new_with_encoding(initial)
    }
}

impl<T, Ser> SharedValue<T, Ser>
where
    T: Debug + SerializableData<Ser>,
    T::SerErr: Debug,
    Ser: Serializer,
{
    pub fn new_with_encoding(initial: impl FnOnce() -> T) -> Self {
        let value: T;
        #[cfg(feature = "hydration")]
        {
            use reactive_graph::owner::Owner;

            let sc = Owner::current_shared_context();
            let id = sc.as_ref().map(|sc| sc.next_id()).unwrap_or_default();
            let serialized = sc.as_ref().and_then(|sc| sc.read_data(&id));
            let hydrating =
                sc.as_ref().map(|sc| sc.during_hydration()).unwrap_or(false);
            value = if hydrating {
                serialized
                    .as_ref()
                    .and_then(|data| T::de(data).ok())
                    .unwrap_or_else(|| {
                        #[cfg(feature = "tracing")]
                        tracing::error!(
                            "couldn't deserialize from {serialized:?}"
                        );
                        initial()
                    })
            } else {
                let init = initial();
                #[cfg(feature = "ssr")]
                if let Some(sc) = sc {
                    match init.ser() {
                        Ok(value) => {
                            sc.write_async(id, Box::pin(async move { value }))
                        }
                        #[allow(unused)] // used in tracing
                        Err(e) => {
                            #[cfg(feature = "tracing")]
                            tracing::error!(
                                "couldn't serialize {init:?}: {e:?}"
                            );
                        }
                    }
                }
                init
            }
        }
        #[cfg(not(feature = "hydration"))]
        {
            value = initial();
        }
        Self {
            value,
            ser: PhantomData,
        }
    }
}

impl<T, Ser> Deref for SharedValue<T, Ser> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T, Ser> DerefMut for SharedValue<T, Ser> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T, Ser> PartialEq for SharedValue<T, Ser>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T, Ser> Eq for SharedValue<T, Ser> where T: Eq {}

impl<T, Ser> Display for SharedValue<T, Ser>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl<T, Ser> Hash for SharedValue<T, Ser>
where
    T: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<T, Ser> PartialOrd for SharedValue<T, Ser>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl<T, Ser> Ord for SharedValue<T, Ser>
where
    T: Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}
