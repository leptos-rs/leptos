// #[wasm_bindgen(thread_local)] is deprecated in wasm-bindgen 0.2.96
// but the replacement is also only shipped in that version
// as a result, we'll just allow deprecated for now
#![allow(deprecated)]

use super::{SerializedDataId, SharedContext};
use crate::{PinnedFuture, PinnedStream};
use core::fmt::Debug;
use js_sys::Array;
use once_cell::sync::Lazy;
use std::{
    fmt::Display,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};
use throw_error::{Error, ErrorId};
use wasm_bindgen::{prelude::wasm_bindgen, JsCast};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(thread_local)]
    static __RESOLVED_RESOURCES: Array;

    #[wasm_bindgen(thread_local)]
    static __SERIALIZED_ERRORS: Array;

    #[wasm_bindgen(thread_local)]
    static __INCOMPLETE_CHUNKS: Array;
}

fn serialized_errors() -> Vec<(SerializedDataId, ErrorId, Error)> {
    __SERIALIZED_ERRORS.with(|s| {
        s.iter()
            .flat_map(|value| {
                value.dyn_ref::<Array>().map(|value| {
                    let error_boundary_id =
                        value.get(0).as_f64().unwrap() as usize;
                    let error_id = value.get(1).as_f64().unwrap() as usize;
                    let value = value
                        .get(2)
                        .as_string()
                        .expect("Expected a [number, string] tuple");
                    (
                        SerializedDataId(error_boundary_id),
                        ErrorId::from(error_id),
                        Error::from(SerializedError(value)),
                    )
                })
            })
            .collect()
    })
}

fn incomplete_chunks() -> Vec<SerializedDataId> {
    __INCOMPLETE_CHUNKS.with(|i| {
        i.iter()
            .map(|value| {
                let id = value.as_f64().unwrap() as usize;
                SerializedDataId(id)
            })
            .collect()
    })
}

/// An error that has been serialized across the network boundary.
#[derive(Debug, Clone)]
struct SerializedError(String);

impl Display for SerializedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl std::error::Error for SerializedError {}

#[derive(Default)]
/// The shared context that should be used in the browser while hydrating.
pub struct HydrateSharedContext {
    id: AtomicUsize,
    is_hydrating: AtomicBool,
    during_hydration: AtomicBool,
    errors: Lazy<Vec<(SerializedDataId, ErrorId, Error)>>,
    incomplete: Lazy<Vec<SerializedDataId>>,
}

impl HydrateSharedContext {
    /// Creates a new shared context for hydration in the browser.
    pub fn new() -> Self {
        Self {
            id: AtomicUsize::new(0),
            is_hydrating: AtomicBool::new(true),
            during_hydration: AtomicBool::new(true),
            errors: Lazy::new(serialized_errors),
            incomplete: Lazy::new(incomplete_chunks),
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
            during_hydration: AtomicBool::new(true),
            errors: Lazy::new(serialized_errors),
            incomplete: Lazy::new(incomplete_chunks),
        }
    }
}

impl Debug for HydrateSharedContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HydrateSharedContext").finish()
    }
}

impl SharedContext for HydrateSharedContext {
    fn is_browser(&self) -> bool {
        true
    }

    fn next_id(&self) -> SerializedDataId {
        let id = self.id.fetch_add(1, Ordering::Relaxed);
        SerializedDataId(id)
    }

    fn write_async(&self, _id: SerializedDataId, _fut: PinnedFuture<String>) {}

    fn read_data(&self, id: &SerializedDataId) -> Option<String> {
        __RESOLVED_RESOURCES.with(|r| r.get(id.0 as u32).as_string())
    }

    fn await_data(&self, _id: &SerializedDataId) -> Option<String> {
        todo!()
    }

    fn pending_data(&self) -> Option<PinnedStream<String>> {
        None
    }

    fn during_hydration(&self) -> bool {
        self.during_hydration.load(Ordering::Relaxed)
    }

    fn hydration_complete(&self) {
        self.during_hydration.store(false, Ordering::Relaxed)
    }

    fn get_is_hydrating(&self) -> bool {
        self.is_hydrating.load(Ordering::Relaxed)
    }

    fn set_is_hydrating(&self, is_hydrating: bool) {
        self.is_hydrating.store(is_hydrating, Ordering::Relaxed)
    }

    fn errors(&self, boundary_id: &SerializedDataId) -> Vec<(ErrorId, Error)> {
        self.errors
            .iter()
            .filter_map(|(boundary, id, error)| {
                if boundary == boundary_id {
                    Some((id.clone(), error.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    #[inline(always)]
    fn register_error(
        &self,
        _error_boundary: SerializedDataId,
        _error_id: ErrorId,
        _error: Error,
    ) {
    }

    #[inline(always)]
    fn seal_errors(&self, _boundary_id: &SerializedDataId) {}

    fn take_errors(&self) -> Vec<(SerializedDataId, ErrorId, Error)> {
        self.errors.clone()
    }

    #[inline(always)]
    fn defer_stream(&self, _wait_for: PinnedFuture<()>) {}

    #[inline(always)]
    fn await_deferred(&self) -> Option<PinnedFuture<()>> {
        None
    }

    #[inline(always)]
    fn set_incomplete_chunk(&self, _id: SerializedDataId) {}

    fn get_incomplete_chunk(&self, id: &SerializedDataId) -> bool {
        self.incomplete.iter().any(|entry| entry == id)
    }
}
