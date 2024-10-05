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
use hydration_context::{SerializedDataId, SharedContext};
use reactive_graph::{
    computed::{
        ArcAsyncDerived, ArcMemo, AsyncDerived, AsyncDerivedFuture,
        AsyncDerivedRefFuture,
    },
    graph::{Source, ToAnySubscriber},
    owner::Owner,
    prelude::*,
    signal::{ArcRwSignal, RwSignal},
};
use std::{
    future::{pending, IntoFuture},
    ops::Deref,
    panic::Location,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub(crate) static IS_SUPPRESSING_RESOURCE_LOAD: AtomicBool =
    AtomicBool::new(false);

pub struct SuppressResourceLoad;

impl SuppressResourceLoad {
    pub fn new() -> Self {
        IS_SUPPRESSING_RESOURCE_LOAD.store(true, Ordering::Relaxed);
        Self
    }
}

impl Default for SuppressResourceLoad {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SuppressResourceLoad {
    fn drop(&mut self) {
        IS_SUPPRESSING_RESOURCE_LOAD.store(false, Ordering::Relaxed);
    }
}

pub struct ArcResource<T, Ser = JsonSerdeCodec> {
    ser: PhantomData<Ser>,
    refetch: ArcRwSignal<usize>,
    data: ArcAsyncDerived<T>,
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
}

impl<T, Ser> Debug for ArcResource<T, Ser> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("ArcResource");
        d.field("ser", &self.ser).field("data", &self.data);
        #[cfg(debug_assertions)]
        d.field("defined_at", self.defined_at);
        d.finish_non_exhaustive()
    }
}

impl<T, Ser> DefinedAt for ArcResource<T, Ser> {
    fn defined_at(&self) -> Option<&'static Location<'static>> {
        #[cfg(debug_assertions)]
        {
            Some(self.defined_at)
        }
        #[cfg(not(debug_assertions))]
        {
            None
        }
    }
}

impl<T, Ser> Clone for ArcResource<T, Ser> {
    fn clone(&self) -> Self {
        Self {
            ser: self.ser,
            refetch: self.refetch.clone(),
            data: self.data.clone(),
            #[cfg(debug_assertions)]
            defined_at: self.defined_at,
        }
    }
}

impl<T, Ser> Deref for ArcResource<T, Ser> {
    type Target = ArcAsyncDerived<T>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T, Ser> Track for ArcResource<T, Ser>
where
    T: 'static,
{
    fn track(&self) {
        self.data.track();
    }
}

impl<T, Ser> ReadUntracked for ArcResource<T, Ser>
where
    T: 'static,
{
    type Value = <ArcAsyncDerived<T> as ReadUntracked>::Value;

    #[track_caller]
    fn try_read_untracked(&self) -> Option<Self::Value> {
        #[cfg(all(feature = "hydration", debug_assertions))]
        {
            use reactive_graph::{
                computed::suspense::SuspenseContext, owner::use_context,
            };
            let suspense = use_context::<SuspenseContext>();
            if suspense.is_none() {
                let location = std::panic::Location::caller();
                reactive_graph::log_warning(format_args!(
                    "At {location}, you are reading a resource in `hydrate` \
                     mode outside a <Suspense/> or <Transition/>. This can \
                     cause hydration mismatch errors and loses out on a \
                     significant performance optimization. To fix this issue, \
                     you can either: \n1. Wrap the place where you read the \
                     resource in a <Suspense/> or <Transition/> component, or \
                     \n2. Switch to using ArcLocalResource::new(), which will \
                     wait to load the resource until the app is hydrated on \
                     the client side. (This will have worse performance in \
                     most cases.)",
                ));
            }
        }
        self.data.try_read_untracked()
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

        let initial = initial_value::<T, Ser>(&id, shared_context.as_ref());
        let is_ready = initial.is_some();

        let refetch = ArcRwSignal::new(0);
        let source = ArcMemo::new({
            let refetch = refetch.clone();
            move |_| (refetch.get(), source())
        });
        let fun = {
            let source = source.clone();
            move || {
                let (_, source) = source.get();
                let fut = fetcher(source);
                async move {
                    if IS_SUPPRESSING_RESOURCE_LOAD.load(Ordering::Relaxed) {
                        pending().await
                    } else {
                        fut.await
                    }
                }
            }
        };

        let data = ArcAsyncDerived::new_with_manual_dependencies(
            initial, fun, &source,
        );
        if is_ready {
            source.with_untracked(|_| ());
            source.add_subscriber(data.to_any_subscriber());
        }

        #[cfg(feature = "ssr")]
        if let Some(shared_context) = shared_context {
            let value = data.clone();
            let ready_fut = data.ready();

            if blocking {
                shared_context.defer_stream(Box::pin(data.ready()));
            }

            if shared_context.get_is_hydrating() {
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
        }

        ArcResource {
            ser: PhantomData,
            data,
            refetch,
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
        }
    }

    #[track_caller]
    pub fn map<U>(&self, f: impl FnOnce(&T) -> U) -> Option<U>
    where
        T: Send + Sync + 'static,
    {
        self.data.try_with(|n| n.as_ref().map(f))?
    }

    /// Re-runs the async function with the current source data.
    pub fn refetch(&self) {
        *self.refetch.write() += 1;
    }
}

#[inline(always)]
#[allow(unused)]
pub(crate) fn initial_value<T, Ser>(
    id: &SerializedDataId,
    shared_context: Option<&Arc<dyn SharedContext + Send + Sync>>,
) -> Option<T>
where
    Ser: Encoder<T> + Decoder<T>,
    <Ser as Encoder<T>>::Error: Debug,
    <Ser as Decoder<T>>::Error: Debug,
    <<Ser as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError: Debug,
    <Ser as Encoder<T>>::Encoded: IntoEncodedString,
    <Ser as Decoder<T>>::Encoded: FromEncodedStr,
{
    #[cfg(feature = "hydration")]
    {
        use std::borrow::Borrow;

        let shared_context = Owner::current_shared_context();
        if let Some(shared_context) = shared_context {
            let value = shared_context.read_data(id);
            if let Some(value) = value {
                let encoded =
                    match <Ser as Decoder<T>>::Encoded::from_encoded_str(&value)
                    {
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

impl<T, E, Ser> ArcResource<Result<T, E>, Ser>
where
    Ser: Encoder<Result<T, E>> + Decoder<Result<T, E>>,
    <Ser as Encoder<Result<T, E>>>::Error: Debug,
    <Ser as Decoder<Result<T, E>>>::Error: Debug,
    <<Ser as Decoder<Result<T, E>>>::Encoded as FromEncodedStr>::DecodingError:
        Debug,
    <Ser as Encoder<Result<T, E>>>::Encoded: IntoEncodedString,
    <Ser as Decoder<Result<T, E>>>::Encoded: FromEncodedStr,
    T: Send + Sync + 'static,
    E: Send + Sync + Clone + 'static,
{
    #[track_caller]
    pub fn and_then<U>(&self, f: impl FnOnce(&T) -> U) -> Option<Result<U, E>> {
        self.map(|data| data.as_ref().map(f).map_err(|e| e.clone()))
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
    refetch: RwSignal<usize>,
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
}

impl<T, Ser> Debug for Resource<T, Ser>
where
    T: Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("ArcResource");
        d.field("ser", &self.ser).field("data", &self.data);
        #[cfg(debug_assertions)]
        d.field("defined_at", self.defined_at);
        d.finish_non_exhaustive()
    }
}

impl<T, Ser> DefinedAt for Resource<T, Ser>
where
    T: Send + Sync + 'static,
{
    fn defined_at(&self) -> Option<&'static Location<'static>> {
        #[cfg(debug_assertions)]
        {
            Some(self.defined_at)
        }
        #[cfg(not(debug_assertions))]
        {
            None
        }
    }
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

impl<T, Ser> Track for Resource<T, Ser>
where
    T: Send + Sync + 'static,
{
    fn track(&self) {
        self.data.track();
    }
}

impl<T, Ser> ReadUntracked for Resource<T, Ser>
where
    T: Send + Sync + 'static,
{
    type Value = <AsyncDerived<T> as ReadUntracked>::Value;

    #[track_caller]
    fn try_read_untracked(&self) -> Option<Self::Value> {
        #[cfg(all(feature = "hydration", debug_assertions))]
        {
            use reactive_graph::{
                computed::suspense::SuspenseContext, owner::use_context,
            };
            let suspense = use_context::<SuspenseContext>();
            if suspense.is_none() {
                let location = std::panic::Location::caller();
                reactive_graph::log_warning(format_args!(
                    "At {location}, you are reading a resource in `hydrate` \
                     mode outside a <Suspense/> or <Transition/>. This can \
                     cause hydration mismatch errors and loses out on a \
                     significant performance optimization. To fix this issue, \
                     you can either: \n1. Wrap the place where you read the \
                     resource in a <Suspense/> or <Transition/> component, or \
                     \n2. Switch to using LocalResource::new(), which will \
                     wait to load the resource until the app is hydrated on \
                     the client side. (This will have worse performance in \
                     most cases.)",
                ));
            }
        }
        self.data.try_read_untracked()
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
        let ArcResource { data, refetch, .. }: ArcResource<T, Ser> =
            ArcResource::new_with_options(source, fetcher, blocking);
        Resource {
            ser: PhantomData,
            data: data.into(),
            refetch: refetch.into(),
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
        }
    }

    pub fn map<U>(&self, f: impl FnOnce(&T) -> U) -> Option<U> {
        self.data
            .try_with(|n| n.as_ref().map(|n| Some(f(n))))?
            .flatten()
    }

    /// Re-runs the async function with the current source data.
    pub fn refetch(&self) {
        self.refetch.try_update(|n| *n += 1);
    }
}

impl<T, E, Ser> Resource<Result<T, E>, Ser>
where
    Ser: Encoder<Result<T, E>> + Decoder<Result<T, E>>,
    <Ser as Encoder<Result<T, E>>>::Error: Debug,
    <Ser as Decoder<Result<T, E>>>::Error: Debug,
    <<Ser as Decoder<Result<T, E>>>::Encoded as FromEncodedStr>::DecodingError:
        Debug,
    <Ser as Encoder<Result<T, E>>>::Encoded: IntoEncodedString,
    <Ser as Decoder<Result<T, E>>>::Encoded: FromEncodedStr,
    T: Send + Sync,
    E: Send + Sync + Clone,
{
    #[track_caller]
    pub fn and_then<U>(&self, f: impl FnOnce(&T) -> U) -> Option<Result<U, E>> {
        self.map(|data| data.as_ref().map(f).map_err(|e| e.clone()))
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
