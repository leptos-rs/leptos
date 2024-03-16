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
    /// be deserialized using [`Serializable::de`].
    ///
    /// On the server and in client-side rendered implementations, this should
    /// always return [`None`].
    fn read_data(&self, id: &SerializedDataId) -> Option<String>;

    /// Returns a [`Future`] that resolves with a `String` that should
    /// be deserialized using [`Serializable::de`] once the given piece of server
    /// data has resolved.
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
}
