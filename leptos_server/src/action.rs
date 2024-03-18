use reactive_graph::{
    action::ArcAction,
    owner::StoredValue,
    signal::{ArcReadSignal, ArcRwSignal, ReadSignal, RwSignal},
    traits::DefinedAt,
    unwrap_signal,
};
use server_fn::{error::ServerFnUrlError, ServerFn, ServerFnError};
use std::panic::Location;

pub struct ArcServerAction<S>
where
    S: ServerFn + 'static,
    S::Output: 'static,
{
    inner: ArcAction<S, Result<S::Output, ServerFnError<S::Error>>>,
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
}

impl<S> ArcServerAction<S>
where
    S: ServerFn + Clone + Send + Sync + 'static,
    S::Output: Send + Sync + 'static,
    S::Error: Send + Sync + 'static,
{
    #[track_caller]
    pub fn new() -> Self {
        Self {
            inner: ArcAction::new(|input: &S| S::run_on_client(input.clone())),
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
        }
    }

    #[track_caller]
    pub fn dispatch(&self, input: S) {
        self.inner.dispatch(input);
    }
}

impl<S> ArcServerAction<S>
where
    S: ServerFn + Clone + Send + Sync + 'static,
    S::Output: 'static,
    S::Error: 'static,
{
    #[track_caller]
    pub fn version(&self) -> ArcRwSignal<usize> {
        self.inner.version()
    }

    #[track_caller]
    pub fn input(&self) -> ArcRwSignal<Option<S>> {
        self.inner.input()
    }

    #[track_caller]
    pub fn value(
        &self,
    ) -> ArcRwSignal<Option<Result<S::Output, ServerFnError<S::Error>>>> {
        self.inner.value()
    }
}

impl<S> Clone for ArcServerAction<S>
where
    S: ServerFn + 'static,
    S::Output: 'static,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            #[cfg(debug_assertions)]
            defined_at: self.defined_at,
        }
    }
}

impl<S> Default for ArcServerAction<S>
where
    S: ServerFn + Clone + Send + Sync + 'static,
    S::Output: Send + Sync + 'static,
    S::Error: Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S> DefinedAt for ArcServerAction<S>
where
    S: ServerFn + 'static,
    S::Output: 'static,
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

pub struct ServerAction<S>
where
    S: ServerFn + 'static,
    S::Output: 'static,
{
    inner: StoredValue<ArcServerAction<S>>,
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
}

impl<S> Clone for ServerAction<S>
where
    S: ServerFn + 'static,
    S::Output: 'static,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for ServerAction<S>
where
    S: ServerFn + 'static,
    S::Output: 'static,
{
}

impl<S> ServerAction<S>
where
    S: ServerFn + Clone + Send + Sync + 'static,
    S::Output: Send + Sync + 'static,
    S::Error: Send + Sync + 'static,
{
    #[track_caller]
    pub fn new() -> Self {
        Self {
            inner: StoredValue::new(ArcServerAction::new()),
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
        }
    }

    #[track_caller]
    pub fn dispatch(&self, input: S) {
        self.inner.with_value(|inner| inner.dispatch(input));
    }

    #[track_caller]
    pub fn version(&self) -> RwSignal<usize> {
        self.inner
            .with_value(|inner| inner.version())
            .unwrap_or_else(unwrap_signal!(self))
            .into()
    }

    #[track_caller]
    pub fn input(&self) -> RwSignal<Option<S>> {
        self.inner
            .with_value(|inner| inner.input())
            .unwrap_or_else(unwrap_signal!(self))
            .into()
    }

    #[track_caller]
    pub fn value(
        &self,
    ) -> RwSignal<Option<Result<S::Output, ServerFnError<S::Error>>>> {
        self.inner
            .with_value(|inner| inner.value())
            .unwrap_or_else(unwrap_signal!(self))
            .into()
    }
}

impl<S> Default for ServerAction<S>
where
    S: ServerFn + Clone + Send + Sync + 'static,
    S::Output: Send + Sync + 'static,
    S::Error: Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S> DefinedAt for ServerAction<S>
where
    S: ServerFn + 'static,
    S::Output: 'static,
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
