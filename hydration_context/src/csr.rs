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
}
