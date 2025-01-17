//! Isomorphic web applications that run on the server to render HTML, then add interactivity in
//! the client, need to accomplish two tasks:
//! 1. Send HTML from the server, so that the client can "hydrate" it in the browser by adding
//!    event listeners and setting up other interactivity.
//! 2. Send data that was loaded on the server to the client, so that the client "hydrates" with
//!    the same data with which the server rendered HTML.
//!
//! This crate helps with the second part of this process. It provides a [`SharedContext`] type
//! that allows you to store data on the server, and then extract the same data in the client.

#![deny(missing_docs)]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "browser")]
#[cfg_attr(docsrs, doc(cfg(feature = "browser")))]
mod csr;
#[cfg(feature = "browser")]
#[cfg_attr(docsrs, doc(cfg(feature = "browser")))]
mod hydrate;
mod ssr;
#[cfg(feature = "browser")]
pub use csr::*;
use futures::Stream;
#[cfg(feature = "browser")]
pub use hydrate::*;
use serde::{Deserialize, Serialize};
pub use ssr::*;
use std::{fmt::Debug, future::Future, pin::Pin};
use throw_error::{Error, ErrorId};

/// Type alias for a boxed [`Future`].
pub type PinnedFuture<T> = Pin<Box<dyn Future<Output = T> + Send + Sync>>;
/// Type alias for a boxed [`Future`] that is `!Send`.
pub type PinnedLocalFuture<T> = Pin<Box<dyn Future<Output = T>>>;
/// Type alias for a boxed [`Stream`].
pub type PinnedStream<T> = Pin<Box<dyn Stream<Item = T> + Send + Sync>>;

#[derive(
    Clone, Debug, PartialEq, Eq, Hash, Default, Deserialize, Serialize,
)]
#[serde(transparent)]
/// A unique identifier for a piece of data that will be serialized
/// from the server to the client.
pub struct SerializedDataId(usize);

impl SerializedDataId {
    /// Create a new instance of [`SerializedDataId`].
    pub fn new(id: usize) -> Self {
        SerializedDataId(id)
    }

    /// Consume into the inner usize identifier.
    pub fn into_inner(self) -> usize {
        self.0
    }
}

impl From<SerializedDataId> for ErrorId {
    fn from(value: SerializedDataId) -> Self {
        value.0.into()
    }
}

/// Information that will be shared between the server and the client.
pub trait SharedContext: Debug {
    /// Whether the application is running in the browser.
    fn is_browser(&self) -> bool;

    /// Returns the next in a series of IDs that is unique to a particular request and response.
    ///
    /// This should not be used as a global unique ID mechanism. It is specific to the process
    /// of serializing and deserializing data from the server to the browser as part of an HTTP
    /// response.
    fn next_id(&self) -> SerializedDataId;

    /// The given [`Future`] should resolve with some data that can be serialized
    /// from the server to the client. This will be polled as part of the process of
    /// building the HTTP response, *not* when it is first created.
    ///
    /// In browser implementations, this should be a no-op.
    fn write_async(&self, id: SerializedDataId, fut: PinnedFuture<String>);

    /// Reads the current value of some data from the shared context, if it has been
    /// sent from the server. This returns the serialized data as a `String` that should
    /// be deserialized.
    ///
    /// On the server and in client-side rendered implementations, this should
    /// always return [`None`].
    fn read_data(&self, id: &SerializedDataId) -> Option<String>;

    /// Returns a [`Future`] that resolves with a `String` that should
    /// be deserialized once the given piece of server data has resolved.
    ///
    /// On the server and in client-side rendered implementations, this should
    /// return a [`Future`] that is immediately ready with [`None`].
    fn await_data(&self, id: &SerializedDataId) -> Option<String>;

    /// Returns some [`Stream`] of HTML that contains JavaScript `<script>` tags defining
    /// all values being serialized from the server to the client, with their serialized values
    /// and any boilerplate needed to notify a running application that they exist; or `None`.
    ///
    /// In browser implementations, this return `None`.
    fn pending_data(&self) -> Option<PinnedStream<String>>;

    /// Whether the page is currently being hydrated.
    ///
    /// Should always be `false` on the server or when client-rendering, including after the
    /// initial hydration in the client.
    fn during_hydration(&self) -> bool;

    /// Tells the shared context that the hydration process is complete.
    fn hydration_complete(&self);

    /// Returns `true` if you are currently in a part of the application tree that should be
    /// hydrated.
    ///
    /// For example, in an app with "islands," this should be `true` inside islands and
    /// false elsewhere.
    fn get_is_hydrating(&self) -> bool;

    /// Sets whether you are currently in a part of the application tree that should be hydrated.
    ///
    /// For example, in an app with "islands," this should be `true` inside islands and
    /// false elsewhere.
    fn set_is_hydrating(&self, is_hydrating: bool);

    /// Returns all errors that have been registered, removing them from the list.
    fn take_errors(&self) -> Vec<(SerializedDataId, ErrorId, Error)>;

    /// Returns the set of errors that have been registered with a particular boundary.
    fn errors(&self, boundary_id: &SerializedDataId) -> Vec<(ErrorId, Error)>;

    /// "Seals" an error boundary, preventing further errors from being registered for it.
    ///
    /// This can be used in streaming SSR scenarios in which the final state of the error boundary
    /// can only be known after the initial state is hydrated.
    fn seal_errors(&self, boundary_id: &SerializedDataId);

    /// Registers an error with the context to be shared from server to client.
    fn register_error(
        &self,
        error_boundary: SerializedDataId,
        error_id: ErrorId,
        error: Error,
    );

    /// Adds a `Future` to the set of “blocking resources” that should prevent the server’s
    /// response stream from beginning until all are resolved. The `Future` returned by
    /// blocking resources will not resolve until every `Future` added by this method
    /// has resolved.
    ///
    /// In browser implementations, this should be a no-op.
    fn defer_stream(&self, wait_for: PinnedFuture<()>);

    /// Returns a `Future` that will resolve when every `Future` added via
    /// [`defer_stream`](Self::defer_stream) has resolved.
    ///
    /// In browser implementations, this should be a no-op.
    fn await_deferred(&self) -> Option<PinnedFuture<()>>;

    /// Tells the client that this chunk is being sent from the server before all its data have
    /// loaded, and it may be in a fallback state.
    fn set_incomplete_chunk(&self, id: SerializedDataId);

    /// Checks whether this chunk is being sent from the server before all its data have loaded.
    fn get_incomplete_chunk(&self, id: &SerializedDataId) -> bool;
}
