use reactive_graph::{
    actions::{Action, ArcAction},
    owner::use_context,
    traits::DefinedAt,
};
use server_fn::{error::ServerFnErrorSerde, ServerFn, ServerFnError};
use std::{ops::Deref, panic::Location, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerActionError {
    path: Arc<str>,
    err: Arc<str>,
}

impl ServerActionError {
    pub fn new(path: &str, err: &str) -> Self {
        Self {
            path: path.into(),
            err: err.into(),
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn err(&self) -> &str {
        &self.err
    }
}

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
        let err = use_context::<ServerActionError>().and_then(|error| {
            (error.path() == S::PATH)
                .then(|| ServerFnError::<S::Error>::de(error.err()))
                .map(Err)
        });
        Self {
            inner: ArcAction::new_with_value(err, |input: &S| {
                S::run_on_client(input.clone())
            }),
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
        let err = use_context::<ServerActionError>().and_then(|error| {
            (error.path() == S::PATH)
                .then(|| ServerFnError::<S::Error>::de(error.err()))
                .map(Err)
        });
        Self {
            inner: Action::new_with_value(err, |input: &S| {
                S::run_on_client(input.clone())
            }),
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
