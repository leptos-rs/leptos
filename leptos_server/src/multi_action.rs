use reactive_graph::{
    actions::{ArcMultiAction, MultiAction},
    traits::DefinedAt,
};
use server_fn::{ServerFn, ServerFnError};
use std::{ops::Deref, panic::Location};

pub struct ArcServerMultiAction<S>
where
    S: ServerFn + 'static,
    S::Output: 'static,
{
    inner: ArcMultiAction<S, Result<S::Output, ServerFnError<S::Error>>>,
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
}

impl<S> ArcServerMultiAction<S>
where
    S: ServerFn + Clone + Send + Sync + 'static,
    S::Output: Send + Sync + 'static,
    S::Error: Send + Sync + 'static,
{
    #[track_caller]
    pub fn new() -> Self {
        Self {
            inner: ArcMultiAction::new(|input: &S| {
                S::run_on_client(input.clone())
            }),
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
        }
    }
}

impl<S> Deref for ArcServerMultiAction<S>
where
    S: ServerFn + 'static,
    S::Output: 'static,
{
    type Target = ArcMultiAction<S, Result<S::Output, ServerFnError<S::Error>>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<S> Clone for ArcServerMultiAction<S>
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

impl<S> Default for ArcServerMultiAction<S>
where
    S: ServerFn + Clone + Send + Sync + 'static,
    S::Output: Send + Sync + 'static,
    S::Error: Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S> DefinedAt for ArcServerMultiAction<S>
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

pub struct ServerMultiAction<S>
where
    S: ServerFn + 'static,
    S::Output: 'static,
{
    inner: MultiAction<S, Result<S::Output, ServerFnError<S::Error>>>,
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
}

impl<S> From<ServerMultiAction<S>>
    for MultiAction<S, Result<S::Output, ServerFnError<S::Error>>>
where
    S: ServerFn + 'static,
    S::Output: 'static,
{
    fn from(value: ServerMultiAction<S>) -> Self {
        value.inner
    }
}

impl<S> ServerMultiAction<S>
where
    S: ServerFn + Send + Sync + Clone + 'static,
    S::Output: Send + Sync + 'static,
    S::Error: Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            inner: MultiAction::new(|input: &S| {
                S::run_on_client(input.clone())
            }),
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
        }
    }
}

impl<S> Clone for ServerMultiAction<S>
where
    S: ServerFn + 'static,
    S::Output: 'static,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for ServerMultiAction<S>
where
    S: ServerFn + 'static,
    S::Output: 'static,
{
}

impl<S> Deref for ServerMultiAction<S>
where
    S: ServerFn + 'static,
    S::Output: 'static,
    S::Error: 'static,
{
    type Target = MultiAction<S, Result<S::Output, ServerFnError<S::Error>>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<S> Default for ServerMultiAction<S>
where
    S: ServerFn + Clone + Send + Sync + 'static,
    S::Output: Send + Sync + 'static,
    S::Error: Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S> DefinedAt for ServerMultiAction<S>
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
