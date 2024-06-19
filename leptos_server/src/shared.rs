use crate::serializers::{SerdeJson, SerializableData, Serializer};
use std::{
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

/// A smart pointer that allows you to share identical, synchronously-loaded data between the
/// server and the client.
///
/// If this constructed on the server, it serializes its value into the shared context. If it is
/// constructed on the client during hydration, it reads its value from the shared context. If
/// it it constructed on the client at any other time, it simply runs on the client.
pub struct SharedValue<T, Ser = SerdeJson> {
    value: T,
    ser: PhantomData<Ser>,
}

impl<T, Ser> SharedValue<T, Ser> {
    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T, Ser> SharedValue<T, Ser>
where
    T: Debug + SerializableData<Ser>,
    T::SerErr: Debug,
    Ser: Serializer,
{
    pub fn new(initial: impl FnOnce() -> T) -> Self {
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
