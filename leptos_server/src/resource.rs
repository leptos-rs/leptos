use crate::{FromEncodedStr, IntoEncodedString};
#[cfg(feature = "rkyv")]
use codee::binary::RkyvCodec;
#[cfg(feature = "serde-wasm-bindgen")]
use codee::string::JsonSerdeWasmCodec;
#[cfg(feature = "miniserde")]
use codee::string::MiniserdeCodec;
#[cfg(feature = "serde-lite")]
use codee::SerdeLite;
use codee::{
    string::{FromToStringCodec, JsonSerdeCodec},
    Decoder, Encoder,
};
use core::{fmt::Debug, marker::PhantomData};
use futures::Future;
use hydration_context::SerializedDataId;
use reactive_graph::{
    computed::{
        ArcAsyncDerived, ArcMemo, AsyncDerived, AsyncDerivedFuture,
        AsyncDerivedRefFuture,
    },
    graph::{Source, ToAnySubscriber},
    owner::Owner,
    prelude::*,
};
use std::{future::IntoFuture, ops::Deref};

pub struct ArcResource<T, Ser = JsonSerdeCodec> {
    ser: PhantomData<Ser>,
    data: ArcAsyncDerived<T>,
}

impl<T, Ser> Clone for ArcResource<T, Ser> {
    fn clone(&self) -> Self {
        Self {
            ser: self.ser,
            data: self.data.clone(),
        }
    }
}

impl<T, Ser> Deref for ArcResource<T, Ser> {
    type Target = ArcAsyncDerived<T>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T, Ser> ArcResource<T, Ser>
where
    Ser: Encoder<T> + Decoder<T>,
    <Ser as Encoder<T>>::Error: Debug,
    <Ser as Decoder<T>>::Error: Debug,
    <<Ser as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError: Debug,
    <Ser as Encoder<T>>::Encoded: IntoEncodedString,
    <Ser as Decoder<T>>::Encoded: FromEncodedStr,
{
    #[track_caller]
    pub fn new_with_options<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
        #[allow(unused)] // this is used with `feature = "ssr"`
        blocking: bool,
    ) -> ArcResource<T, Ser>
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        let shared_context = Owner::current_shared_context();
        let id = shared_context
            .as_ref()
            .map(|sc| sc.next_id())
            .unwrap_or_default();

        let initial = Self::initial_value(&id);
        let is_ready = initial.is_some();

        let source = ArcMemo::new(move |_| source());
        let fun = {
            let source = source.clone();
            move || fetcher(source.get())
        };

        let data =
            ArcAsyncDerived::new_with_initial_without_spawning(initial, fun);
        if is_ready {
            source.with(|_| ());
            source.add_subscriber(data.to_any_subscriber());
        }

        #[cfg(feature = "ssr")]
        if let Some(shared_context) = shared_context {
            let value = data.clone();
            let ready_fut = data.ready();

            if blocking {
                shared_context.defer_stream(Box::pin(data.ready()));
            }

            shared_context.write_async(
                id,
                Box::pin(async move {
                    ready_fut.await;
                    value.with_untracked(|data| match &data {
                        // TODO handle serialization errors
                        Some(val) => {
                            Ser::encode(val).unwrap().into_encoded_string()
                        }
                        _ => unreachable!(),
                    })
                }),
            );
        }

        ArcResource {
            ser: PhantomData,
            data,
        }
    }

    #[inline(always)]
    #[allow(unused)]
    fn initial_value(id: &SerializedDataId) -> Option<T> {
        #[cfg(feature = "hydration")]
        {
            use std::borrow::Borrow;

            let shared_context = Owner::current_shared_context();
            if let Some(shared_context) = shared_context {
                let value = shared_context.read_data(id);
                if let Some(value) = value {
                    let encoded =
                        match <Ser as Decoder<T>>::Encoded::from_encoded_str(
                            &value,
                        ) {
                            Ok(value) => value,
                            Err(e) => {
                                #[cfg(feature = "tracing")]
                                tracing::error!("couldn't deserialize: {e:?}");
                                return None;
                            }
                        };
                    let encoded = encoded.borrow();
                    match Ser::decode(encoded) {
                        Ok(value) => return Some(value),
                        #[allow(unused)]
                        Err(e) => {
                            #[cfg(feature = "tracing")]
                            tracing::error!("couldn't deserialize: {e:?}");
                        }
                    }
                }
            }
        }
        None
    }
}

impl<T> ArcResource<T, JsonSerdeCodec>
where
    JsonSerdeCodec: Encoder<T> + Decoder<T>,
    <JsonSerdeCodec as Encoder<T>>::Error: Debug,
    <JsonSerdeCodec as Decoder<T>>::Error: Debug,
    <<JsonSerdeCodec as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError:
        Debug,
    <JsonSerdeCodec as Encoder<T>>::Encoded: IntoEncodedString,
    <JsonSerdeCodec as Decoder<T>>::Encoded: FromEncodedStr,
{
    #[track_caller]
    pub fn new<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        ArcResource::new_with_options(source, fetcher, false)
    }

    #[track_caller]
    pub fn new_blocking<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        ArcResource::new_with_options(source, fetcher, true)
    }
}

impl<T> ArcResource<T, FromToStringCodec>
where
    FromToStringCodec: Encoder<T> + Decoder<T>,
    <FromToStringCodec as Encoder<T>>::Error: Debug, <FromToStringCodec as Decoder<T>>::Error: Debug,
    <<FromToStringCodec as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError: Debug,
    <FromToStringCodec as Encoder<T>>::Encoded: IntoEncodedString,
    <FromToStringCodec as Decoder<T>>::Encoded: FromEncodedStr,
{
    pub fn new_str<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        ArcResource::new_with_options(source, fetcher, false)
    }

    pub fn new_str_blocking<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        ArcResource::new_with_options(source, fetcher, true)
    }
}

#[cfg(feature = "serde-wasm-bindgen")]
impl<T> ArcResource<T, JsonSerdeWasmCodec>
where
    JsonSerdeWasmCodec: Encoder<T> + Decoder<T>,
    <JsonSerdeWasmCodec as Encoder<T>>::Error: Debug, <JsonSerdeWasmCodec as Decoder<T>>::Error: Debug,
    <<JsonSerdeWasmCodec as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError: Debug,
    <JsonSerdeWasmCodec as Encoder<T>>::Encoded: IntoEncodedString,
    <JsonSerdeWasmCodec as Decoder<T>>::Encoded: FromEncodedStr,
{
    #[track_caller]
    pub fn new_serde_wb<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        ArcResource::new_with_options(source, fetcher, false)
    }

    #[track_caller]
    pub fn new_serde_wb_blocking<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        ArcResource::new_with_options(source, fetcher, true)
    }
}
#[cfg(feature = "miniserde")]
impl<T> ArcResource<T, MiniserdeCodec>
where
    MiniserdeCodec: Encoder<T> + Decoder<T>,
    <MiniserdeCodec as Encoder<T>>::Error: Debug,
    <MiniserdeCodec as Decoder<T>>::Error: Debug,
    <<MiniserdeCodec as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError:
        Debug,
    <MiniserdeCodec as Encoder<T>>::Encoded: IntoEncodedString,
    <MiniserdeCodec as Decoder<T>>::Encoded: FromEncodedStr,
{
    #[track_caller]
    pub fn new_miniserde<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        ArcResource::new_with_options(source, fetcher, false)
    }

    #[track_caller]
    pub fn new_miniserde_blocking<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        ArcResource::new_with_options(source, fetcher, true)
    }
}

#[cfg(feature = "serde-lite")]
impl<T> ArcResource<T, SerdeLite<JsonSerdeCodec>>
where
    SerdeLite<JsonSerdeCodec>: Encoder<T> + Decoder<T>,
    <SerdeLite<JsonSerdeCodec> as Encoder<T>>::Error: Debug, <SerdeLite<JsonSerdeCodec> as Decoder<T>>::Error: Debug,
    <<SerdeLite<JsonSerdeCodec> as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError: Debug,
    <SerdeLite<JsonSerdeCodec> as Encoder<T>>::Encoded: IntoEncodedString,
    <SerdeLite<JsonSerdeCodec> as Decoder<T>>::Encoded: FromEncodedStr,
{
    #[track_caller]
    pub fn new_serde_lite<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        ArcResource::new_with_options(source, fetcher, false)
    }

    #[track_caller]
    pub fn new_serde_lite_blocking<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        ArcResource::new_with_options(source, fetcher, true)
    }
}

#[cfg(feature = "rkyv")]
impl<T> ArcResource<T, RkyvCodec>
where
    RkyvCodec: Encoder<T> + Decoder<T>,
    <RkyvCodec as Encoder<T>>::Error: Debug,
    <RkyvCodec as Decoder<T>>::Error: Debug,
    <<RkyvCodec as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError:
        Debug,
    <RkyvCodec as Encoder<T>>::Encoded: IntoEncodedString,
    <RkyvCodec as Decoder<T>>::Encoded: FromEncodedStr,
{
    #[track_caller]
    pub fn new_rkyv<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        ArcResource::new_with_options(source, fetcher, false)
    }

    #[track_caller]
    pub fn new_rkyv_blocking<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        ArcResource::new_with_options(source, fetcher, true)
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

impl<T, Ser> ArcResource<T, Ser>
where
    T: 'static,
{
    pub fn by_ref(&self) -> AsyncDerivedRefFuture<T> {
        self.data.by_ref()
    }
}

pub struct Resource<T, Ser = JsonSerdeCodec>
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

impl<T> Resource<T, FromToStringCodec>
where
    FromToStringCodec: Encoder<T> + Decoder<T>,
    <FromToStringCodec as Encoder<T>>::Error: Debug, <FromToStringCodec as Decoder<T>>::Error: Debug,
    <<FromToStringCodec as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError: Debug,
    <FromToStringCodec as Encoder<T>>::Encoded: IntoEncodedString,
    <FromToStringCodec as Decoder<T>>::Encoded: FromEncodedStr,
    T: Send + Sync,
{
    #[track_caller]
    pub fn new_str<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        Resource::new_with_options(source, fetcher, false)
    }

    #[track_caller]
    pub fn new_str_blocking<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        Resource::new_with_options(source, fetcher, true)
    }
}

impl<T> Resource<T, JsonSerdeCodec>
where
    JsonSerdeCodec: Encoder<T> + Decoder<T>,
    <JsonSerdeCodec as Encoder<T>>::Error: Debug,
    <JsonSerdeCodec as Decoder<T>>::Error: Debug,
    <<JsonSerdeCodec as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError:
        Debug,
    <JsonSerdeCodec as Encoder<T>>::Encoded: IntoEncodedString,
    <JsonSerdeCodec as Decoder<T>>::Encoded: FromEncodedStr,
    T: Send + Sync,
{
    #[track_caller]
    pub fn new<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        Resource::new_with_options(source, fetcher, false)
    }

    #[track_caller]
    pub fn new_blocking<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        Resource::new_with_options(source, fetcher, true)
    }
}

#[cfg(feature = "serde-wasm-bindgen")]
impl<T> Resource<T, JsonSerdeWasmCodec>
where
    JsonSerdeWasmCodec: Encoder<T> + Decoder<T>,
    <JsonSerdeWasmCodec as Encoder<T>>::Error: Debug, <JsonSerdeWasmCodec as Decoder<T>>::Error: Debug,
    <<JsonSerdeWasmCodec as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError: Debug,
    <JsonSerdeWasmCodec as Encoder<T>>::Encoded: IntoEncodedString,
    <JsonSerdeWasmCodec as Decoder<T>>::Encoded: FromEncodedStr,
    T: Send + Sync,
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
        Resource::new_with_options(source, fetcher, false)
    }

    pub fn new_serde_wb_blocking<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        Resource::new_with_options(source, fetcher, true)
    }
}

#[cfg(feature = "miniserde")]
impl<T> Resource<T, MiniserdeCodec>
where
    MiniserdeCodec: Encoder<T> + Decoder<T>,
    <MiniserdeCodec as Encoder<T>>::Error: Debug,
    <MiniserdeCodec as Decoder<T>>::Error: Debug,
    <<MiniserdeCodec as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError:
        Debug,
    <MiniserdeCodec as Encoder<T>>::Encoded: IntoEncodedString,
    <MiniserdeCodec as Decoder<T>>::Encoded: FromEncodedStr,
    T: Send + Sync,
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
        Resource::new_with_options(source, fetcher, false)
    }
}

#[cfg(feature = "serde-lite")]
impl<T> Resource<T, SerdeLite<JsonSerdeCodec>>
where
    SerdeLite<JsonSerdeCodec>: Encoder<T> + Decoder<T>,
    <SerdeLite<JsonSerdeCodec> as Encoder<T>>::Error: Debug, <SerdeLite<JsonSerdeCodec> as Decoder<T>>::Error: Debug,
    <<SerdeLite<JsonSerdeCodec> as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError:
        Debug,
    <SerdeLite<JsonSerdeCodec> as Encoder<T>>::Encoded: IntoEncodedString,
    <SerdeLite<JsonSerdeCodec> as Decoder<T>>::Encoded: FromEncodedStr,
    T: Send + Sync,
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
        Resource::new_with_options(source, fetcher, false)
    }

    pub fn new_serde_lite_blocking<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        Resource::new_with_options(source, fetcher, true)
    }
}

#[cfg(feature = "rkyv")]
impl<T> Resource<T, RkyvCodec>
where
    RkyvCodec: Encoder<T> + Decoder<T>,
    <RkyvCodec as Encoder<T>>::Error: Debug,
    <RkyvCodec as Decoder<T>>::Error: Debug,
    <<RkyvCodec as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError:
        Debug,
    <RkyvCodec as Encoder<T>>::Encoded: IntoEncodedString,
    <RkyvCodec as Decoder<T>>::Encoded: FromEncodedStr,
    T: Send + Sync,
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
        Resource::new_with_options(source, fetcher, false)
    }

    pub fn new_rkyv_blocking<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
    ) -> Self
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        Resource::new_with_options(source, fetcher, true)
    }
}

impl<T, Ser> Resource<T, Ser>
where
    Ser: Encoder<T> + Decoder<T>,
    <Ser as Encoder<T>>::Error: Debug,
    <Ser as Decoder<T>>::Error: Debug,
    <<Ser as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError: Debug,
    <Ser as Encoder<T>>::Encoded: IntoEncodedString,
    <Ser as Decoder<T>>::Encoded: FromEncodedStr,
    T: Send + Sync,
{
    #[track_caller]
    pub fn new_with_options<S, Fut>(
        source: impl Fn() -> S + Send + Sync + 'static,
        fetcher: impl Fn(S) -> Fut + Send + Sync + 'static,
        blocking: bool,
    ) -> Resource<T, Ser>
    where
        S: Send + Sync + Clone + PartialEq + 'static,
        T: Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        let ArcResource { data, .. }: ArcResource<T, Ser> =
            ArcResource::new_with_options(source, fetcher, blocking);
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

    #[track_caller]
    fn into_future(self) -> Self::IntoFuture {
        self.data.into_future()
    }
}

impl<T, Ser> Resource<T, Ser>
where
    T: Send + Sync + 'static,
{
    pub fn by_ref(&self) -> AsyncDerivedRefFuture<T> {
        self.data.by_ref()
    }
}
