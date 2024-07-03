#[cfg(all(feature = "hydrate", feature = "experimental-islands"))]
use crate::Owner;
use crate::{
    runtime::PinnedFuture, suspense::StreamChunk, with_runtime, ResourceId,
    SignalGet, SuspenseContext,
};
use futures::stream::FuturesUnordered;
#[cfg(feature = "experimental-islands")]
use std::cell::Cell;
use std::collections::{HashMap, HashSet, VecDeque};
#[doc(hidden)]
/// Hydration data and other context that is shared between the server
/// and the client.
pub struct SharedContext {
    /// Resources that initially needed to resolve from the server.
    pub server_resources: HashSet<ResourceId>,
    /// Resources that have not yet resolved.
    pub pending_resources: HashSet<ResourceId>,
    /// Resources that have already resolved.
    pub resolved_resources: HashMap<ResourceId, String>,
    /// Suspended fragments that have not yet resolved.
    pub pending_fragments: HashMap<String, FragmentData>,
    /// Suspense fragments that contain only local resources.
    pub fragments_with_local_resources: HashSet<String>,
    #[cfg(feature = "experimental-islands")]
    pub no_hydrate: bool,
    #[cfg(all(feature = "hydrate", feature = "experimental-islands"))]
    pub islands: HashMap<Owner, web_sys::HtmlElement>,
}

impl SharedContext {
    /// Returns IDs for all [`Resource`](crate::Resource)s found on any scope.
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    pub fn all_resources() -> Vec<ResourceId> {
        with_runtime(|runtime| runtime.all_resources()).unwrap_or_default()
    }

    /// Returns IDs for all [`Resource`](crate::Resource)s found on any scope that are
    /// pending from the server.
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    pub fn pending_resources() -> Vec<ResourceId> {
        with_runtime(|runtime| runtime.pending_resources()).unwrap_or_default()
    }

    /// Returns IDs for all [`Resource`](crate::Resource)s found on any scope.
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    pub fn serialization_resolvers(
    ) -> FuturesUnordered<PinnedFuture<(ResourceId, String)>> {
        with_runtime(|runtime| runtime.serialization_resolvers())
            .unwrap_or_default()
    }

    /// Registers the given [`SuspenseContext`](crate::SuspenseContext) with the current scope,
    /// calling the `resolver` when its resources are all resolved.
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    pub fn register_suspense(
        context: SuspenseContext,
        key: &str,
        out_of_order_resolver: impl FnOnce() -> String + 'static,
        in_order_resolver: impl FnOnce() -> VecDeque<StreamChunk> + 'static,
    ) {
        use crate::create_isomorphic_effect;
        use futures::StreamExt;

        _ = with_runtime(|runtime| {
            let mut shared_context = runtime.shared_context.borrow_mut();
            let (tx1, mut rx1) = futures::channel::mpsc::unbounded();
            let (tx2, mut rx2) = futures::channel::mpsc::unbounded();
            let (tx3, mut rx3) = futures::channel::mpsc::unbounded();

            create_isomorphic_effect(move |_| {
                let pending = context
                    .pending_serializable_resources
                    .read_only()
                    .try_get()
                    .unwrap_or_default();
                if pending.is_empty() {
                    _ = tx1.unbounded_send(());
                    _ = tx2.unbounded_send(());
                    _ = tx3.unbounded_send(());
                }
            });

            shared_context.pending_fragments.insert(
                key.to_string(),
                FragmentData {
                    out_of_order: Box::pin(async move {
                        rx1.next().await;

                        out_of_order_resolver()
                    }),
                    in_order: Box::pin(async move {
                        rx2.next().await;

                        in_order_resolver()
                    }),
                    should_block: context.should_block(),
                    is_ready: Some(Box::pin(async move {
                        rx3.next().await;
                    })),
                    local_only: context.has_local_only(),
                },
            );
        })
    }

    /// Takes the pending HTML for a single `<Suspense/>` node.
    ///
    /// Returns a tuple of two pinned `Future`s that return content for out-of-order
    /// and in-order streaming, respectively.
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    pub fn take_pending_fragment(id: &str) -> Option<FragmentData> {
        with_runtime(|runtime| {
            let mut shared_context = runtime.shared_context.borrow_mut();
            shared_context.pending_fragments.remove(id)
        })
        .ok()
        .flatten()
    }

    /// A future that will resolve when all blocking fragments are ready.
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    pub fn blocking_fragments_ready() -> PinnedFuture<()> {
        use futures::StreamExt;

        let mut ready = with_runtime(|runtime| {
            let mut shared_context = runtime.shared_context.borrow_mut();
            let ready = FuturesUnordered::new();
            for (_, data) in shared_context.pending_fragments.iter_mut() {
                if data.should_block {
                    if let Some(is_ready) = data.is_ready.take() {
                        ready.push(is_ready);
                    }
                }
            }
            ready
        })
        .unwrap_or_default();
        Box::pin(async move { while ready.next().await.is_some() {} })
    }

    /// The set of all HTML fragments currently pending.
    ///
    /// The keys are hydration IDs. Values are tuples of two pinned
    /// `Future`s that return content for out-of-order and in-order streaming, respectively.
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    pub fn pending_fragments() -> HashMap<String, FragmentData> {
        with_runtime(|runtime| {
            let mut shared_context = runtime.shared_context.borrow_mut();
            std::mem::take(&mut shared_context.pending_fragments)
        })
        .unwrap_or_default()
    }

    /// Registers the given element as an island with the current reactive owner.
    #[cfg(all(feature = "hydrate", feature = "experimental-islands"))]
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    pub fn register_island(el: &web_sys::HtmlElement) {
        if let Some(owner) = Owner::current() {
            let el = el.clone();
            _ = with_runtime(|runtime| {
                let mut shared_context = runtime.shared_context.borrow_mut();
                shared_context.islands.insert(owner, el);
            });
        }
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    pub fn fragment_has_local_resources(fragment: &str) -> bool {
        with_runtime(|runtime| {
            let mut shared_context = runtime.shared_context.borrow_mut();
            shared_context
                .fragments_with_local_resources
                .remove(fragment)
        })
        .unwrap_or_default()
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    pub fn fragments_with_local_resources() -> HashSet<String> {
        with_runtime(|runtime| {
            let mut shared_context = runtime.shared_context.borrow_mut();
            std::mem::take(&mut shared_context.fragments_with_local_resources)
        })
        .unwrap_or_default()
    }

    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", skip_all,)
    )]
    pub fn register_local_fragment(key: String) {
        with_runtime(|runtime| {
            let mut shared_context = runtime.shared_context.borrow_mut();
            shared_context.fragments_with_local_resources.insert(key);
        })
        .unwrap_or_default()
    }
}

/// Represents its pending `<Suspense/>` fragment.
pub struct FragmentData {
    /// Future that represents how it should be render for an out-of-order stream.
    pub out_of_order: PinnedFuture<String>,
    /// Future that represents how it should be render for an in-order stream.
    pub in_order: PinnedFuture<VecDeque<StreamChunk>>,
    /// Whether the stream should wait for this fragment before sending any data.
    pub should_block: bool,
    /// Future that will resolve when the fragment is ready.
    pub is_ready: Option<PinnedFuture<()>>,
    /// Whether the fragment contains only local resources.
    pub local_only: bool,
}

impl core::fmt::Debug for SharedContext {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SharedContext").finish()
    }
}

impl PartialEq for SharedContext {
    fn eq(&self, other: &Self) -> bool {
        self.pending_resources == other.pending_resources
            && self.resolved_resources == other.resolved_resources
    }
}

impl Eq for SharedContext {}

#[allow(clippy::derivable_impls)]
impl Default for SharedContext {
    fn default() -> Self {
        #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
        {
            let pending_resources = js_sys::Reflect::get(
                &web_sys::window().unwrap(),
                &wasm_bindgen::JsValue::from_str("__LEPTOS_PENDING_RESOURCES"),
            );
            let pending_resources: HashSet<ResourceId> = pending_resources
                .map_err(|_| ())
                .and_then(|pr| {
                    serde_wasm_bindgen::from_value(pr).map_err(|_| ())
                })
                .unwrap();
            let fragments_with_local_resources = js_sys::Reflect::get(
                &web_sys::window().unwrap(),
                &wasm_bindgen::JsValue::from_str("__LEPTOS_LOCAL_ONLY"),
            );
            let fragments_with_local_resources: HashSet<String> =
                fragments_with_local_resources
                    .map_err(|_| ())
                    .and_then(|pr| {
                        serde_wasm_bindgen::from_value(pr).map_err(|_| ())
                    })
                    .unwrap_or_default();

            let resolved_resources = js_sys::Reflect::get(
                &web_sys::window().unwrap(),
                &wasm_bindgen::JsValue::from_str("__LEPTOS_RESOLVED_RESOURCES"),
            )
            .unwrap(); // unwrap_or(wasm_bindgen::JsValue::NULL);

            let resolved_resources =
                serde_wasm_bindgen::from_value(resolved_resources).unwrap();

            Self {
                server_resources: pending_resources.clone(),
                //events: Default::default(),
                pending_resources,
                resolved_resources,
                fragments_with_local_resources,
                pending_fragments: Default::default(),
                #[cfg(feature = "experimental-islands")]
                no_hydrate: true,
                #[cfg(all(
                    feature = "hydrate",
                    feature = "experimental-islands"
                ))]
                islands: Default::default(),
            }
        }
        #[cfg(not(all(feature = "hydrate", target_arch = "wasm32")))]
        {
            Self {
                server_resources: Default::default(),
                //events: Default::default(),
                pending_resources: Default::default(),
                resolved_resources: Default::default(),
                pending_fragments: Default::default(),
                fragments_with_local_resources: Default::default(),
                #[cfg(feature = "experimental-islands")]
                no_hydrate: true,
                #[cfg(all(
                    feature = "hydrate",
                    feature = "experimental-islands"
                ))]
                islands: Default::default(),
            }
        }
    }
}

#[cfg(feature = "experimental-islands")]
thread_local! {
  pub static NO_HYDRATE: Cell<bool> = const { Cell::new(true) };
}

#[cfg(feature = "experimental-islands")]
impl SharedContext {
    /// Whether the renderer should currently add hydration IDs.
    pub fn no_hydrate() -> bool {
        NO_HYDRATE.with(Cell::get)
    }

    /// Sets whether the renderer should not add hydration IDs.
    pub fn set_no_hydrate(hydrate: bool) {
        NO_HYDRATE.with(|cell| cell.set(hydrate));
    }

    /// Turns on hydration for the duration of the function call
    #[inline(always)]
    pub fn with_hydration<T>(f: impl FnOnce() -> T) -> T {
        let prev = SharedContext::no_hydrate();
        SharedContext::set_no_hydrate(false);
        let v = f();
        SharedContext::set_no_hydrate(prev);
        v
    }

    /// Turns off hydration for the duration of the function call
    #[inline(always)]
    pub fn no_hydration<T>(f: impl FnOnce() -> T) -> T {
        let prev = SharedContext::no_hydrate();
        SharedContext::set_no_hydrate(true);
        let v = f();
        SharedContext::set_no_hydrate(prev);
        v
    }
}
