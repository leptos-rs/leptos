#[cfg(feature = "miniserde")]
use crate::serializers::Miniserde;
#[cfg(feature = "rkyv")]
use crate::serializers::Rkyv;
#[cfg(feature = "serde-lite")]
use crate::serializers::SerdeLite;
#[cfg(feature = "serde-wasm-bindgen")]
use crate::serializers::SerdeWasmBindgen;
use crate::serializers::{SerdeJson, SerializableData, Serializer, Str};
use core::{fmt::Debug, marker::PhantomData};
use futures::Future;
use hydration_context::SerializedDataId;
use reactive_graph::{
    computed::{
        ArcAsyncDerived, ArcMemo, AsyncDerived, AsyncDerivedFuture, AsyncState,
    },
    owner::Owner,
    prelude::*,
};
use std::{future::IntoFuture, ops::Deref};

pub struct ArcResource<T, Ser = SerdeJson> {
    ser: PhantomData<Ser>,
    data: ArcAsyncDerived<T>,
}

impl<T, Ser> Deref for ArcResource<T, Ser> {
    type Target = ArcAsyncDerived<T>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> ArcResource<T, Str>
where
    T: Debug + SerializableData<Str>,
    T::SerErr: Debug,
    T::DeErr: Debug,
{
    pub fn new<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        ArcResource::new_with_encoding(source, fetcher)
    }
}

impl<T> ArcResource<T, SerdeJson>
where
    T: Debug + SerializableData<SerdeJson>,
    T::SerErr: Debug,
    T::DeErr: Debug,
{
    pub fn new_serde<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        ArcResource::new_with_encoding(source, fetcher)
    }
}

#[cfg(feature = "serde-wasm-bindgen")]
impl<T> ArcResource<T, SerdeWasmBindgen>
where
    T: Debug + SerializableData<SerdeWasmBindgen>,
    T::SerErr: Debug,
    T::DeErr: Debug,
{
    pub fn new_serde_wb<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        ArcResource::new_with_encoding(source, fetcher)
    }
}
#[cfg(feature = "miniserde")]
impl<T> ArcResource<T, Miniserde>
where
    T: Debug + SerializableData<Miniserde>,
    T::SerErr: Debug,
    T::DeErr: Debug,
{
    pub fn new_miniserde<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        ArcResource::new_with_encoding(source, fetcher)
    }
}

#[cfg(feature = "serde-lite")]
impl<T> ArcResource<T, SerdeLite>
where
    T: Debug + SerializableData<SerdeLite>,
    T::SerErr: Debug,
    T::DeErr: Debug,
{
    pub fn new_serde_lite<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        ArcResource::new_with_encoding(source, fetcher)
    }
}

#[cfg(feature = "rkyv")]
impl<T> ArcResource<T, SerdeLite>
where
    T: Debug + SerializableData<SerdeLite>,
    T::SerErr: Debug,
    T::DeErr: Debug,
{
    pub fn new_rkyv<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        ArcResource::new_with_encoding(source, fetcher)
    }
}

impl<T, Ser> ArcResource<T, Ser>
where
    Ser: Serializer,
    T: Debug + SerializableData<Ser>,
    T::SerErr: Debug,
    T::DeErr: Debug,
{
    pub fn new_with_encoding<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> ArcResource<T, Ser>
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Debug + Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        let shared_context = Owner::current_shared_context();
        let id = shared_context
            .as_ref()
            .map(|sc| sc.next_id())
            .unwrap_or_default();

        let initial = Self::initial_value(&id);

        let source = ArcMemo::new(move |_| source());
        let fun = move || fetcher(source.get());

        let data = ArcAsyncDerived::new_with_initial(initial, fun);

        if let Some(shared_context) = shared_context {
            let value = data.clone();
            let ready_fut = data.ready();

            shared_context.write_async(
                id,
                Box::pin(async move {
                    ready_fut.await;
                    value
                        .with_untracked(|data| match &data {
                            AsyncState::Complete(val) => val.ser(),
                            _ => unreachable!(),
                        })
                        .unwrap() // TODO handle
                }),
            );
        }

        ArcResource {
            ser: PhantomData,
            data,
        }
    }

    #[inline(always)]
    fn initial_value(id: &SerializedDataId) -> AsyncState<T> {
        #[cfg(feature = "hydration")]
        {
            let shared_context = Owner::current_shared_context();
            if let Some(shared_context) = shared_context {
                let value = shared_context.read_data(id);
                if let Some(value) = value {
                    match T::de(&value) {
                        Ok(value) => return AsyncState::Complete(value),
                        Err(e) => {
                            #[cfg(feature = "tracing")]
                            tracing::error!(
                                "couldn't deserialize from {value:?}: {e:?}"
                            );
                        }
                    }
                }
            }
        }
        AsyncState::Loading
    }
}

impl<T, Ser> IntoFuture for ArcResource<T, Ser>
where
    T: Clone + 'static,
{
    type Output = T;
    type IntoFuture = AsyncDerivedFuture<T>;

    fn into_future(self) -> Self::IntoFuture {
        self.data.into_future()
    }
}

pub struct Resource<T, Ser>
where
    T: Send + Sync + 'static,
{
    ser: PhantomData<Ser>,
    data: AsyncDerived<T>,
}

impl<T: Send + Sync + 'static, Ser> Copy for Resource<T, Ser> {}

impl<T: Send + Sync + 'static, Ser> Clone for Resource<T, Ser> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T, Ser> Deref for Resource<T, Ser>
where
    T: Send + Sync + 'static,
{
    type Target = AsyncDerived<T>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> Resource<T, Str>
where
    T: Debug + SerializableData<Str> + Send + Sync + 'static,
    T::SerErr: Debug,
    T::DeErr: Debug,
{
    pub fn new<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        Resource::new_with_encoding(source, fetcher)
    }
}

impl<T> Resource<T, SerdeJson>
where
    T: Debug + SerializableData<SerdeJson> + Send + Sync + 'static,
    T::SerErr: Debug,
    T::DeErr: Debug,
{
    pub fn new_serde<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        Resource::new_with_encoding(source, fetcher)
    }
}

#[cfg(feature = "serde-wasm-bindgen")]
impl<T> Resource<T, SerdeWasmBindgen>
where
    T: Debug + SerializableData<SerdeWasmBindgen> + Send + Sync + 'static,
    T::SerErr: Debug,
    T::DeErr: Debug,
{
    pub fn new_serde_wb<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        Resource::new_with_encoding(source, fetcher)
    }
}

#[cfg(feature = "miniserde")]
impl<T> Resource<T, Miniserde>
where
    T: Debug + SerializableData<Miniserde> + Send + Sync + 'static,
    T::SerErr: Debug,
    T::DeErr: Debug,
{
    pub fn new_miniserde<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        Resource::new_with_encoding(source, fetcher)
    }
}

#[cfg(feature = "serde-lite")]
impl<T> Resource<T, SerdeLite>
where
    T: Debug + SerializableData<SerdeLite> + Send + Sync + 'static,
    T::SerErr: Debug,
    T::DeErr: Debug,
{
    pub fn new_serde_lite<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        Resource::new_with_encoding(source, fetcher)
    }
}

#[cfg(feature = "rkyv")]
impl<T> Resource<T, Rkyv>
where
    T: Debug + SerializableData<Rkyv> + Send + Sync + 'static,
    T::SerErr: Debug,
    T::DeErr: Debug,
{
    pub fn new_rkyv<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        Resource::new_with_encoding(source, fetcher)
    }
}

impl<T, Ser> Resource<T, Ser>
where
    Ser: Serializer,
    T: Debug + SerializableData<Ser> + Send + Sync + 'static,
    T::SerErr: Debug,
    T::DeErr: Debug,
{
    pub fn new_with_encoding<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Resource<T, Ser>
    where
        S: Send + Sync + Clone + PartialEq + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        let ArcResource { ser, data } =
            ArcResource::new_with_encoding(source, fetcher);
        Resource {
            ser: PhantomData,
            data: data.into(),
        }
    }
}

impl<T, Ser> IntoFuture for Resource<T, Ser>
where
    T: Clone + Send + Sync + 'static,
{
    type Output = T;
    type IntoFuture = AsyncDerivedFuture<T>;

    fn into_future(self) -> Self::IntoFuture {
        self.data.into_future()
    }
}
