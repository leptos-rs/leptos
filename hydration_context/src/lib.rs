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
use std::{
    fmt::{Debug, Display},
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
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
///
/// IDs are either top-level (like `3`), or nested within some other ID (like
/// `3-0`). Allocations inside a deferred subtree — a suspense boundary's
/// resolution, a `Suspend`ed future — extend that subtree's anchor, so the
/// same resource receives the same identifier on the server and in the
/// browser regardless of the order in which deferred subtrees run.
pub struct SerializedDataId(String);

impl SerializedDataId {
    /// Create a new root-level instance of [`SerializedDataId`].
    pub fn new(id: usize) -> Self {
        SerializedDataId(id.to_string())
    }

    /// Returns the ID as a string, which is used as its key when serialized.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume into the inner identifier.
    pub fn into_inner(self) -> String {
        self.0
    }

    fn child(&self, n: usize) -> Self {
        if self.0.is_empty() {
            SerializedDataId(n.to_string())
        } else {
            SerializedDataId(format!("{}-{n}", self.0))
        }
    }

    /// The anchor for the tree's root scope: its children are the plain
    /// top-level identifiers (`"0"`, `"1"`, …). The browser's hydration walk
    /// enters a scope anchored here so that walk allocations mirror the
    /// server's top-level sequence even when unrelated client-side work runs
    /// between the walk's await points.
    pub fn root_anchor() -> Self {
        SerializedDataId(String::new())
    }

    /// A browser-local identifier that can never collide with an identifier
    /// the server serialized. Allocations that happen outside the hydration
    /// walk — post-hydration mounts, effect re-runs at the walk's await
    /// points — have no serialized data to read, so they must not consume
    /// identifiers from the walk's sequence.
    pub fn browser_local(n: usize) -> Self {
        SerializedDataId(format!("c{n}"))
    }
}

impl Display for SerializedDataId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<SerializedDataId> for ErrorId {
    fn from(value: SerializedDataId) -> Self {
        value.0.into()
    }
}

thread_local! {
    static ID_SCOPES: std::cell::RefCell<Vec<IdScopeFrame>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

enum IdScopeFrame {
    /// Allocations extend the scope's parent with its shared sequence.
    Anchored(SerializedDataIdScope),
    /// Allocations are handed throwaway identifiers that are never
    /// serialized — used for server-side discovery walks, which re-run view
    /// closures whose creations must not consume real identifiers.
    Throwaway,
}

/// Restores the previous id-allocation scope when dropped.
pub struct IdScopeGuard(());

impl Drop for IdScopeGuard {
    fn drop(&mut self) {
        ID_SCOPES.with(|scopes| {
            scopes.borrow_mut().pop();
        });
    }
}

/// Enters the given id-allocation scope: until the returned guard is dropped,
/// identifiers allocated on this thread come from `scope`.
pub fn enter_id_scope(scope: SerializedDataIdScope) -> IdScopeGuard {
    ID_SCOPES.with(|scopes| {
        scopes.borrow_mut().push(IdScopeFrame::Anchored(scope))
    });
    IdScopeGuard(())
}

/// Enters a scope in which allocated identifiers are throwaway: they are
/// syntactically valid but can never collide with serialized data. Server-side
/// discovery walks re-run view closures, and the resources those re-runs
/// create must not consume identifiers the client will also allocate.
pub fn enter_throwaway_id_scope() -> IdScopeGuard {
    ID_SCOPES.with(|scopes| {
        scopes.borrow_mut().push(IdScopeFrame::Throwaway)
    });
    IdScopeGuard(())
}

pub(crate) fn scoped_next_id() -> Option<SerializedDataId> {
    ID_SCOPES.with(|scopes| {
        let scopes = scopes.borrow();
        match scopes.last()? {
            IdScopeFrame::Anchored(scope) => Some(scope.next_id()),
            IdScopeFrame::Throwaway => {
                Some(SerializedDataId("discard".to_string()))
            }
        }
    })
}

pin_project_lite::pin_project! {
    /// Allocates serialized-data identifiers from `scope` during every poll of
    /// the wrapped future, so that data created inside a deferred subtree
    /// receives the same identifiers on the server and in the browser no
    /// matter when the subtree actually runs. Clones of a scope share one
    /// sequence, so the numbering continues across several futures and
    /// synchronous phases that belong to the same subtree.
    pub struct IdScopedFuture<Fut> {
        scope: SerializedDataIdScope,
        #[pin]
        inner: Fut,
    }
}

impl<Fut> IdScopedFuture<Fut> {
    /// Wraps the future so its allocations come from `scope`.
    pub fn new(scope: SerializedDataIdScope, inner: Fut) -> Self {
        Self { scope, inner }
    }
}

impl<Fut> Future for IdScopedFuture<Fut>
where
    Fut: Future,
{
    type Output = Fut::Output;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();
        let _guard = enter_id_scope(this.scope.clone());
        this.inner.poll(cx)
    }
}

/// Allocates [`SerializedDataId`]s nested within some parent ID.
///
/// A `<Suspense/>` boundary may create resources after it has waited for some async data. On the
/// server, sibling boundaries resolve in whatever order their data happen to load, but in the
/// browser they are hydrated in the order in which they appear in the tree. Allocating IDs from
/// a scope that belongs to each boundary means that the IDs created inside one boundary do not
/// depend on the timing of any other boundary.
#[derive(Debug, Clone)]
pub struct SerializedDataIdScope {
    parent: SerializedDataId,
    next: Arc<AtomicUsize>,
}

impl SerializedDataIdScope {
    /// Creates a new scope, in which IDs are nested within the given parent ID.
    pub fn new(parent: SerializedDataId) -> Self {
        Self {
            parent,
            next: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Returns the next ID in this scope.
    pub fn next_id(&self) -> SerializedDataId {
        self.parent.child(self.next.fetch_add(1, Ordering::Relaxed))
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

    /// Returns a [`Future`] that resolves once hydration has completed, or
    /// `None` when no deferral is needed — on the server, when
    /// client-rendering, or once hydration is already complete.
    ///
    /// Effects created while the page is hydrating await this before their
    /// first run. The hydration walk matches the browser tree against the
    /// server-rendered HTML, and an effect that runs at one of the walk's
    /// await points can flip state the server never saw — the walk then
    /// builds a different branch than the server rendered and fails with a
    /// hydration mismatch.
    fn hydration_barrier(&self) -> Option<PinnedFuture<()>> {
        None
    }

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
