use reactive_graph::{
    actions::{Action, ArcAction},
    traits::DefinedAt,
};
use server_fn::{ServerFn, ServerFnError};
use std::{ops::Deref, panic::Location};

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
}

impl<S> Deref for ArcServerAction<S>
where
    S: ServerFn + 'static,
    S::Output: 'static,
{
    type Target = ArcAction<S, Result<S::Output, ServerFnError<S::Error>>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
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
    inner: Action<S, Result<S::Output, ServerFnError<S::Error>>>,
    #[cfg(debug_assertions)]
    defined_at: &'static Location<'static>,
}

impl<S> ServerAction<S>
where
    S: ServerFn + Send + Sync + Clone + 'static,
    S::Output: Send + Sync + 'static,
    S::Error: Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            inner: Action::new(|input: &S| S::run_on_client(input.clone())),
            #[cfg(debug_assertions)]
            defined_at: Location::caller(),
        }
    }
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

impl<S> Deref for ServerAction<S>
where
    S: ServerFn + Clone + Send + Sync + 'static,
    S::Output: Send + Sync + 'static,
    S::Error: Send + Sync + 'static,
{
    type Target = Action<S, Result<S::Output, ServerFnError<S::Error>>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<S> From<ServerAction<S>>
    for Action<S, Result<S::Output, ServerFnError<S::Error>>>
where
    S: ServerFn + 'static,
    S::Output: 'static,
{
    fn from(value: ServerAction<S>) -> Self {
        value.inner
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
