use crate::{
    initial_value, FromEncodedStr, IntoEncodedString,
    IS_SUPPRESSING_RESOURCE_LOAD,
};
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
use or_poisoned::OrPoisoned;
use reactive_graph::{
    computed::{
        suspense::SuspenseContext, AsyncDerivedReadyFuture, ScopedFuture,
    },
    diagnostics::{SpecialNonReactiveFuture, SpecialNonReactiveZone},
    graph::{AnySource, ToAnySource},
    owner::{use_context, ArenaItem, Owner},
    prelude::*,
    signal::{
        guards::{Plain, ReadGuard},
        ArcTrigger,
    },
    unwrap_signal,
};
use std::{
    future::IntoFuture,
    mem,
    panic::Location,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
    task::{Context, Poll, Waker},
};

#[derive(Debug)]
pub struct ArcOnceResource<T, Ser = JsonSerdeCodec> {
    trigger: ArcTrigger,
    value: Arc<RwLock<Option<T>>>,
    wakers: Arc<RwLock<Vec<Waker>>>,
    suspenses: Arc<RwLock<Vec<SuspenseContext>>>,
    loading: Arc<AtomicBool>,
    ser: PhantomData<fn() -> Ser>,
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
}

impl<T, Ser> Clone for ArcOnceResource<T, Ser> {
    fn clone(&self) -> Self {
        Self {
            trigger: self.trigger.clone(),
            value: self.value.clone(),
            wakers: self.wakers.clone(),
            suspenses: self.suspenses.clone(),
            loading: self.loading.clone(),
            ser: self.ser,
            #[cfg(debug_assertions)]
            defined_at: self.defined_at,
        }
    }
}

impl<T, Ser> ArcOnceResource<T, Ser>
where
    T: Send + Sync + 'static,
    Ser: Encoder<T> + Decoder<T>,
    <Ser as Encoder<T>>::Error: Debug,
    <Ser as Decoder<T>>::Error: Debug,
    <<Ser as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError: Debug,
    <Ser as Encoder<T>>::Encoded: IntoEncodedString,
    <Ser as Decoder<T>>::Encoded: FromEncodedStr,
{
    #[track_caller]
    pub fn new_with_options(
        fut: impl Future<Output = T> + Send + 'static,
        #[allow(unused)] // this is used with `feature = "ssr"`
        blocking: bool,
    ) -> Self {
        let shared_context = Owner::current_shared_context();
        let id = shared_context
            .as_ref()
            .map(|sc| sc.next_id())
            .unwrap_or_default();

        let initial = initial_value::<T, Ser>(&id, shared_context.as_ref());
        let is_ready = initial.is_some();
        let value = Arc::new(RwLock::new(initial));
        let wakers = Arc::new(RwLock::new(Vec::<Waker>::new()));
        let suspenses = Arc::new(RwLock::new(Vec::<SuspenseContext>::new()));
        let loading = Arc::new(AtomicBool::new(!is_ready));
        let trigger = ArcTrigger::new();

        let fut = ScopedFuture::new(fut);

        if !is_ready && !IS_SUPPRESSING_RESOURCE_LOAD.load(Ordering::Relaxed) {
            let value = Arc::clone(&value);
            let wakers = Arc::clone(&wakers);
            let loading = Arc::clone(&loading);
            let trigger = trigger.clone();
            reactive_graph::spawn(async move {
                let loaded = fut.await;
                *value.write().or_poisoned() = Some(loaded);
                loading.store(false, Ordering::Relaxed);
                for waker in mem::take(&mut *wakers.write().or_poisoned()) {
                    waker.wake();
                }
                trigger.notify();
            });
        }

        let data = Self {
            trigger,
            value: value.clone(),
            loading,
            wakers,
            suspenses,
            ser: PhantomData,
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
        };

        #[cfg(feature = "ssr")]
        if let Some(shared_context) = shared_context {
            let value = Arc::clone(&value);
            let ready_fut = data.ready();

            if blocking {
                shared_context.defer_stream(Box::pin(data.ready()));
            }

            if shared_context.get_is_hydrating() {
                shared_context.write_async(
                    id,
                    Box::pin(async move {
                        ready_fut.await;
                        let value = value.read().or_poisoned();
                        let value = value.as_ref().unwrap();
                        Ser::encode(value).unwrap().into_encoded_string()
                    }),
                );
            }
        }

        data
    }
}

impl<T, Ser> ArcOnceResource<T, Ser> {
    /// Returns a `Future` that is ready when this resource has next finished loading.
    pub fn ready(&self) -> AsyncDerivedReadyFuture {
        AsyncDerivedReadyFuture::new(
            self.to_any_source(),
            &self.loading,
            &self.wakers,
        )
    }
}

impl<T, Ser> DefinedAt for ArcOnceResource<T, Ser> {
    fn defined_at(&self) -> Option<&'static Location<'static>> {
        #[cfg(not(debug_assertions))]
        {
            None
        }
        #[cfg(debug_assertions)]
        {
            Some(self.defined_at)
        }
    }
}

impl<T, Ser> IsDisposed for ArcOnceResource<T, Ser> {
    #[inline(always)]
    fn is_disposed(&self) -> bool {
        false
    }
}

impl<T, Ser> ToAnySource for ArcOnceResource<T, Ser> {
    fn to_any_source(&self) -> AnySource {
        self.trigger.to_any_source()
    }
}

impl<T, Ser> Track for ArcOnceResource<T, Ser> {
    fn track(&self) {
        self.trigger.track();
    }
}

impl<T, Ser> ReadUntracked for ArcOnceResource<T, Ser>
where
    T: 'static,
{
    type Value = ReadGuard<Option<T>, Plain<Option<T>>>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        if let Some(suspense_context) = use_context::<SuspenseContext>() {
            if self.value.read().or_poisoned().is_none() {
                let handle = suspense_context.task_id();
                let ready = SpecialNonReactiveFuture::new(self.ready());
                reactive_graph::spawn(async move {
                    ready.await;
                    drop(handle);
                });
                self.suspenses.write().or_poisoned().push(suspense_context);
            }
        }
        Plain::try_new(Arc::clone(&self.value)).map(ReadGuard::new)
    }
}

impl<T, Ser> IntoFuture for ArcOnceResource<T, Ser>
where
    T: Clone + 'static,
{
    type Output = T;
    type IntoFuture = OnceResourceFuture<T>;

    fn into_future(self) -> Self::IntoFuture {
        OnceResourceFuture {
            source: self.to_any_source(),
            value: Arc::clone(&self.value),
            loading: Arc::clone(&self.loading),
            wakers: Arc::clone(&self.wakers),
            suspenses: Arc::clone(&self.suspenses),
        }
    }
}

/// A [`Future`] that is ready when an [`ArcAsyncDerived`] is finished loading or reloading,
/// and contains its value. `.await`ing this clones the value `T`.
pub struct OnceResourceFuture<T> {
    source: AnySource,
    value: Arc<RwLock<Option<T>>>,
    loading: Arc<AtomicBool>,
    wakers: Arc<RwLock<Vec<Waker>>>,
    suspenses: Arc<RwLock<Vec<SuspenseContext>>>,
}

impl<T> Future for OnceResourceFuture<T>
where
    T: Clone + 'static,
{
    type Output = T;

    #[track_caller]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        #[cfg(debug_assertions)]
        let _guard = SpecialNonReactiveZone::enter();
        let waker = cx.waker();
        self.source.track();

        if let Some(suspense_context) = use_context::<SuspenseContext>() {
            self.suspenses.write().or_poisoned().push(suspense_context);
        }

        if self.loading.load(Ordering::Relaxed) {
            self.wakers.write().or_poisoned().push(waker.clone());
            Poll::Pending
        } else {
            Poll::Ready(
                self.value.read().or_poisoned().as_ref().unwrap().clone(),
            )
        }
    }
}

impl<T> ArcOnceResource<T, JsonSerdeCodec>
where
    T: Send + Sync + 'static,
    JsonSerdeCodec: Encoder<T> + Decoder<T>,
    <JsonSerdeCodec as Encoder<T>>::Error: Debug,
    <JsonSerdeCodec as Decoder<T>>::Error: Debug,
    <<JsonSerdeCodec as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError:
        Debug,
    <JsonSerdeCodec as Encoder<T>>::Encoded: IntoEncodedString,
    <JsonSerdeCodec as Decoder<T>>::Encoded: FromEncodedStr,
{
    #[track_caller]
    pub fn new(fut: impl Future<Output = T> + Send + 'static) -> Self {
        ArcOnceResource::new_with_options(fut, false)
    }

    #[track_caller]
    pub fn new_blocking(fut: impl Future<Output = T> + Send + 'static) -> Self {
        ArcOnceResource::new_with_options(fut, true)
    }
}

impl<T> ArcOnceResource<T, FromToStringCodec>
where
T: Send + Sync + 'static,
    FromToStringCodec: Encoder<T> + Decoder<T>,
    <FromToStringCodec as Encoder<T>>::Error: Debug, <FromToStringCodec as Decoder<T>>::Error: Debug,
    <<FromToStringCodec as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError: Debug,
    <FromToStringCodec as Encoder<T>>::Encoded: IntoEncodedString,
    <FromToStringCodec as Decoder<T>>::Encoded: FromEncodedStr,
{
    pub fn new_str(
        fut: impl Future<Output = T> + Send + 'static
    ) -> Self
    {
        ArcOnceResource::new_with_options(fut, false)
    }

    pub fn new_str_blocking(
        fut: impl Future<Output = T> + Send + 'static
    ) -> Self
    {
        ArcOnceResource::new_with_options(fut, true)
    }
}

#[cfg(feature = "serde-wasm-bindgen")]
impl<T> ArcOnceResource<T, JsonSerdeWasmCodec>
where
T: Send + Sync + 'static,
    JsonSerdeWasmCodec: Encoder<T> + Decoder<T>,
    <JsonSerdeWasmCodec as Encoder<T>>::Error: Debug, <JsonSerdeWasmCodec as Decoder<T>>::Error: Debug,
    <<JsonSerdeWasmCodec as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError: Debug,
    <JsonSerdeWasmCodec as Encoder<T>>::Encoded: IntoEncodedString,
    <JsonSerdeWasmCodec as Decoder<T>>::Encoded: FromEncodedStr,
{
    #[track_caller]
    pub fn new_serde_wb(
fut: impl Future<Output = T> + Send + 'static
    ) -> Self
    {
        ArcOnceResource::new_with_options(fut, false)
    }

    #[track_caller]
    pub fn new_serde_wb_blocking(
fut: impl Future<Output = T> + Send + 'static
    ) -> Self
    {
        ArcOnceResource::new_with_options(fut, true)
    }
}
#[cfg(feature = "miniserde")]
impl<T> ArcOnceResource<T, MiniserdeCodec>
where
    T: Send + Sync + 'static,
    MiniserdeCodec: Encoder<T> + Decoder<T>,
    <MiniserdeCodec as Encoder<T>>::Error: Debug,
    <MiniserdeCodec as Decoder<T>>::Error: Debug,
    <<MiniserdeCodec as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError:
        Debug,
    <MiniserdeCodec as Encoder<T>>::Encoded: IntoEncodedString,
    <MiniserdeCodec as Decoder<T>>::Encoded: FromEncodedStr,
{
    #[track_caller]
    pub fn new_miniserde(
        fut: impl Future<Output = T> + Send + 'static,
    ) -> Self {
        ArcOnceResource::new_with_options(fut, false)
    }

    #[track_caller]
    pub fn new_miniserde_blocking(
        fut: impl Future<Output = T> + Send + 'static,
    ) -> Self {
        ArcOnceResource::new_with_options(fut, true)
    }
}

#[cfg(feature = "serde-lite")]
impl<T> ArcOnceResource<T, SerdeLite<JsonSerdeCodec>>
where
T: Send + Sync + 'static,
    SerdeLite<JsonSerdeCodec>: Encoder<T> + Decoder<T>,
    <SerdeLite<JsonSerdeCodec> as Encoder<T>>::Error: Debug, <SerdeLite<JsonSerdeCodec> as Decoder<T>>::Error: Debug,
    <<SerdeLite<JsonSerdeCodec> as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError: Debug,
    <SerdeLite<JsonSerdeCodec> as Encoder<T>>::Encoded: IntoEncodedString,
    <SerdeLite<JsonSerdeCodec> as Decoder<T>>::Encoded: FromEncodedStr,
{
    #[track_caller]
    pub fn new_serde_lite(
fut: impl Future<Output = T> + Send + 'static
    ) -> Self
    {
        ArcOnceResource::new_with_options(fut, false)
    }

    #[track_caller]
    pub fn new_serde_lite_blocking(
fut: impl Future<Output = T> + Send + 'static
    ) -> Self
    {
        ArcOnceResource::new_with_options(fut, true)
    }
}

#[cfg(feature = "rkyv")]
impl<T> ArcOnceResource<T, RkyvCodec>
where
    T: Send + Sync + 'static,
    RkyvCodec: Encoder<T> + Decoder<T>,
    <RkyvCodec as Encoder<T>>::Error: Debug,
    <RkyvCodec as Decoder<T>>::Error: Debug,
    <<RkyvCodec as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError:
        Debug,
    <RkyvCodec as Encoder<T>>::Encoded: IntoEncodedString,
    <RkyvCodec as Decoder<T>>::Encoded: FromEncodedStr,
{
    #[track_caller]
    pub fn new_rkyv(fut: impl Future<Output = T> + Send + 'static) -> Self {
        ArcOnceResource::new_with_options(fut, false)
    }

    #[track_caller]
    pub fn new_rkyv_blocking(
        fut: impl Future<Output = T> + Send + 'static,
    ) -> Self {
        ArcOnceResource::new_with_options(fut, true)
    }
}

#[derive(Debug)]
pub struct OnceResource<T, Ser = JsonSerdeCodec> {
    inner: ArenaItem<ArcOnceResource<T, Ser>>,
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
}

impl<T, Ser> Clone for OnceResource<T, Ser> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T, Ser> Copy for OnceResource<T, Ser> {}

impl<T, Ser> OnceResource<T, Ser>
where
    T: Send + Sync + 'static,
    Ser: Encoder<T> + Decoder<T>,
    <Ser as Encoder<T>>::Error: Debug,
    <Ser as Decoder<T>>::Error: Debug,
    <<Ser as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError: Debug,
    <Ser as Encoder<T>>::Encoded: IntoEncodedString,
    <Ser as Decoder<T>>::Encoded: FromEncodedStr,
{
    #[track_caller]
    pub fn new_with_options(
        fut: impl Future<Output = T> + Send + 'static,
        blocking: bool,
    ) -> Self {
        #[cfg(debug_assertions)]
        let defined_at = Location::caller();
        Self {
            inner: ArenaItem::new(ArcOnceResource::new_with_options(
                fut, blocking,
            )),
            #[cfg(debug_assertions)]
            defined_at,
        }
    }
}

impl<T, Ser> OnceResource<T, Ser>
where
    T: Send + Sync + 'static,
    Ser: 'static,
{
    /// Returns a `Future` that is ready when this resource has next finished loading.
    pub fn ready(&self) -> AsyncDerivedReadyFuture {
        self.inner
            .try_with_value(|inner| inner.ready())
            .unwrap_or_else(unwrap_signal!(self))
    }
}

impl<T, Ser> DefinedAt for OnceResource<T, Ser> {
    fn defined_at(&self) -> Option<&'static Location<'static>> {
        #[cfg(not(debug_assertions))]
        {
            None
        }
        #[cfg(debug_assertions)]
        {
            Some(self.defined_at)
        }
    }
}

impl<T, Ser> IsDisposed for OnceResource<T, Ser> {
    #[inline(always)]
    fn is_disposed(&self) -> bool {
        false
    }
}

impl<T, Ser> ToAnySource for OnceResource<T, Ser>
where
    T: Send + Sync + 'static,
    Ser: 'static,
{
    fn to_any_source(&self) -> AnySource {
        self.inner
            .try_with_value(|inner| inner.to_any_source())
            .unwrap_or_else(unwrap_signal!(self))
    }
}

impl<T, Ser> Track for OnceResource<T, Ser>
where
    T: Send + Sync + 'static,
    Ser: 'static,
{
    fn track(&self) {
        if let Some(inner) = self.inner.try_get_value() {
            inner.track();
        }
    }
}

impl<T, Ser> ReadUntracked for OnceResource<T, Ser>
where
    T: Send + Sync + 'static,
    Ser: 'static,
{
    type Value = ReadGuard<Option<T>, Plain<Option<T>>>;

    fn try_read_untracked(&self) -> Option<Self::Value> {
        self.inner
            .try_with_value(|inner| inner.try_read_untracked())
            .flatten()
    }
}

impl<T, Ser> IntoFuture for OnceResource<T, Ser>
where
    T: Clone + Send + Sync + 'static,
    Ser: 'static,
{
    type Output = T;
    type IntoFuture = OnceResourceFuture<T>;

    fn into_future(self) -> Self::IntoFuture {
        self.inner
            .try_get_value()
            .unwrap_or_else(unwrap_signal!(self))
            .into_future()
    }
}

impl<T> OnceResource<T, JsonSerdeCodec>
where
    T: Send + Sync + 'static,
    JsonSerdeCodec: Encoder<T> + Decoder<T>,
    <JsonSerdeCodec as Encoder<T>>::Error: Debug,
    <JsonSerdeCodec as Decoder<T>>::Error: Debug,
    <<JsonSerdeCodec as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError:
        Debug,
    <JsonSerdeCodec as Encoder<T>>::Encoded: IntoEncodedString,
    <JsonSerdeCodec as Decoder<T>>::Encoded: FromEncodedStr,
{
    #[track_caller]
    pub fn new(fut: impl Future<Output = T> + Send + 'static) -> Self {
        OnceResource::new_with_options(fut, false)
    }

    #[track_caller]
    pub fn new_blocking(fut: impl Future<Output = T> + Send + 'static) -> Self {
        OnceResource::new_with_options(fut, true)
    }
}

impl<T> OnceResource<T, FromToStringCodec>
where
T: Send + Sync + 'static,
    FromToStringCodec: Encoder<T> + Decoder<T>,
    <FromToStringCodec as Encoder<T>>::Error: Debug, <FromToStringCodec as Decoder<T>>::Error: Debug,
    <<FromToStringCodec as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError: Debug,
    <FromToStringCodec as Encoder<T>>::Encoded: IntoEncodedString,
    <FromToStringCodec as Decoder<T>>::Encoded: FromEncodedStr,
{
    pub fn new_str(
        fut: impl Future<Output = T> + Send + 'static
    ) -> Self
    {
        OnceResource::new_with_options(fut, false)
    }

    pub fn new_str_blocking(
        fut: impl Future<Output = T> + Send + 'static
    ) -> Self
    {
        OnceResource::new_with_options(fut, true)
    }
}

#[cfg(feature = "serde-wasm-bindgen")]
impl<T> OnceResource<T, JsonSerdeWasmCodec>
where
T: Send + Sync + 'static,
    JsonSerdeWasmCodec: Encoder<T> + Decoder<T>,
    <JsonSerdeWasmCodec as Encoder<T>>::Error: Debug, <JsonSerdeWasmCodec as Decoder<T>>::Error: Debug,
    <<JsonSerdeWasmCodec as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError: Debug,
    <JsonSerdeWasmCodec as Encoder<T>>::Encoded: IntoEncodedString,
    <JsonSerdeWasmCodec as Decoder<T>>::Encoded: FromEncodedStr,
{
    #[track_caller]
    pub fn new_serde_wb(
fut: impl Future<Output = T> + Send + 'static
    ) -> Self
    {
        OnceResource::new_with_options(fut, false)
    }

    #[track_caller]
    pub fn new_serde_wb_blocking(
fut: impl Future<Output = T> + Send + 'static
    ) -> Self
    {
        OnceResource::new_with_options(fut, true)
    }
}
#[cfg(feature = "miniserde")]
impl<T> OnceResource<T, MiniserdeCodec>
where
    T: Send + Sync + 'static,
    MiniserdeCodec: Encoder<T> + Decoder<T>,
    <MiniserdeCodec as Encoder<T>>::Error: Debug,
    <MiniserdeCodec as Decoder<T>>::Error: Debug,
    <<MiniserdeCodec as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError:
        Debug,
    <MiniserdeCodec as Encoder<T>>::Encoded: IntoEncodedString,
    <MiniserdeCodec as Decoder<T>>::Encoded: FromEncodedStr,
{
    #[track_caller]
    pub fn new_miniserde(
        fut: impl Future<Output = T> + Send + 'static,
    ) -> Self {
        OnceResource::new_with_options(fut, false)
    }

    #[track_caller]
    pub fn new_miniserde_blocking(
        fut: impl Future<Output = T> + Send + 'static,
    ) -> Self {
        OnceResource::new_with_options(fut, true)
    }
}

#[cfg(feature = "serde-lite")]
impl<T> OnceResource<T, SerdeLite<JsonSerdeCodec>>
where
T: Send + Sync + 'static,
    SerdeLite<JsonSerdeCodec>: Encoder<T> + Decoder<T>,
    <SerdeLite<JsonSerdeCodec> as Encoder<T>>::Error: Debug, <SerdeLite<JsonSerdeCodec> as Decoder<T>>::Error: Debug,
    <<SerdeLite<JsonSerdeCodec> as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError: Debug,
    <SerdeLite<JsonSerdeCodec> as Encoder<T>>::Encoded: IntoEncodedString,
    <SerdeLite<JsonSerdeCodec> as Decoder<T>>::Encoded: FromEncodedStr,
{
    #[track_caller]
    pub fn new_serde_lite(
fut: impl Future<Output = T> + Send + 'static
    ) -> Self
    {
        OnceResource::new_with_options(fut, false)
    }

    #[track_caller]
    pub fn new_serde_lite_blocking(
fut: impl Future<Output = T> + Send + 'static
    ) -> Self
    {
        OnceResource::new_with_options(fut, true)
    }
}

#[cfg(feature = "rkyv")]
impl<T> OnceResource<T, RkyvCodec>
where
    T: Send + Sync + 'static,
    RkyvCodec: Encoder<T> + Decoder<T>,
    <RkyvCodec as Encoder<T>>::Error: Debug,
    <RkyvCodec as Decoder<T>>::Error: Debug,
    <<RkyvCodec as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError:
        Debug,
    <RkyvCodec as Encoder<T>>::Encoded: IntoEncodedString,
    <RkyvCodec as Decoder<T>>::Encoded: FromEncodedStr,
{
    #[track_caller]
    pub fn new_rkyv(fut: impl Future<Output = T> + Send + 'static) -> Self {
        OnceResource::new_with_options(fut, false)
    }

    #[track_caller]
    pub fn new_rkyv_blocking(
        fut: impl Future<Output = T> + Send + 'static,
    ) -> Self {
        OnceResource::new_with_options(fut, true)
    }
}
