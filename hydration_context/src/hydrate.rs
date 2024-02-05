use super::{SerializedDataId, SharedContext};
use crate::{PinnedFuture, PinnedStream};
use core::fmt::Debug;
use js_sys::Array;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
extern "C" {
    static __RESOLVED_RESOURCES: Array;
}

#[derive(Default)]
/// The shared context that should be used in the browser while hydrating.
pub struct HydrateSharedContext {
    id: AtomicUsize,
    is_hydrating: AtomicBool,
}

impl HydrateSharedContext {
    /// Creates a new shared context for hydration in the browser.
    pub fn new() -> Self {
        Self {
            id: AtomicUsize::new(0),
            is_hydrating: AtomicBool::new(true),
        }
    }

    /// Creates a new shared context for hydration in the browser.
    ///
    /// This defaults to a mode in which the app is not hydrated, but allows you to opt into
    /// hydration for certain portions using [`SharedContext::set_is_hydrating`].
    pub fn new_islands() -> Self {
        Self {
            id: AtomicUsize::new(0),
            is_hydrating: AtomicBool::new(false),
        }
    }
}

impl Debug for HydrateSharedContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HydrateSharedContext").finish()
    }
}

impl SharedContext for HydrateSharedContext {
    fn next_id(&self) -> SerializedDataId {
        let id = self.id.fetch_add(1, Ordering::Relaxed);
        SerializedDataId(id)
    }

    fn write_async(&self, _id: SerializedDataId, _fut: PinnedFuture<String>) {}

    fn read_data(&self, id: &SerializedDataId) -> Option<String> {
        __RESOLVED_RESOURCES.get(id.0 as u32).as_string()
    }

    fn await_data(&self, _id: &SerializedDataId) -> Option<String> {
        todo!()
    }

    fn pending_data(&self) -> Option<PinnedStream<String>> {
        None
    }

    fn get_is_hydrating(&self) -> bool {
        self.is_hydrating.load(Ordering::Relaxed)
    }

    fn set_is_hydrating(&self, is_hydrating: bool) {
        self.is_hydrating.store(true, Ordering::Relaxed)
    }
}
