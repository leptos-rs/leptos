use super::{SerializedDataId, SharedContext};
use crate::{PinnedFuture, PinnedStream};

#[derive(Debug, Default)]
/// The shared context that should be used in the browser while hydrating.
pub struct CsrSharedContext;

impl SharedContext for CsrSharedContext {
    #[inline(always)]
    fn is_browser(&self) -> bool {
        true
    }

    #[inline(always)]
    fn next_id(&self) -> SerializedDataId {
        SerializedDataId(0)
    }

    #[inline(always)]
    fn write_async(&self, _id: SerializedDataId, _fut: PinnedFuture<String>) {}

    #[inline(always)]
    fn read_data(&self, _id: &SerializedDataId) -> Option<String> {
        None
    }

    #[inline(always)]
    fn await_data(&self, _id: &SerializedDataId) -> Option<String> {
        todo!()
    }

    #[inline(always)]
    fn pending_data(&self) -> Option<PinnedStream<String>> {
        None
    }

    #[inline(always)]
    fn get_is_hydrating(&self) -> bool {
        false
    }

    #[inline(always)]
    fn set_is_hydrating(&self, _is_hydrating: bool) {}

    #[inline(always)]
    fn errors(
        &self,
        _boundary_id: &SerializedDataId,
    ) -> Vec<(throw_error::ErrorId, throw_error::Error)> {
        Vec::new()
    }

    #[inline(always)]
    fn take_errors(
        &self,
    ) -> Vec<(SerializedDataId, throw_error::ErrorId, throw_error::Error)> {
        Vec::new()
    }

    #[inline(always)]
    fn register_error(
        &self,
        _error_boundary: SerializedDataId,
        _error_id: throw_error::ErrorId,
        _error: throw_error::Error,
    ) {
    }

    #[inline(always)]
    fn seal_errors(&self, _boundary_id: &SerializedDataId) {}

    #[inline(always)]
    fn during_hydration(&self) -> bool {
        false
    }

    #[inline(always)]
    fn hydration_complete(&self) {}

    #[inline(always)]
    fn defer_stream(&self, _wait_for: PinnedFuture<()>) {}

    #[inline(always)]
    fn await_deferred(&self) -> Option<PinnedFuture<()>> {
        None
    }

    #[inline(always)]
    fn set_incomplete_chunk(&self, _id: SerializedDataId) {}

    #[inline(always)]
    fn get_incomplete_chunk(&self, _id: &SerializedDataId) -> bool {
        false
    }
}
