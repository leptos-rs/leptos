use crate::TopoId;
use leptos_reactive::Scope;
use smallvec::{smallvec, SmallVec};
use std::{
  cell::{OnceCell, RefCell},
  rc::Rc,
};

type ParentOffsetSums = Vec<(usize, usize)>;

#[thread_local]
static mut IS_HYDRATING: bool = true;

#[thread_local]
static mut ID: usize = 0;

#[thread_local]
static mut PARENT_OFFSET_SUMS: ParentOffsetSums = vec![];

pub(crate) struct HydrationCtx;

impl HydrationCtx {
  pub(crate) fn id() -> usize {
    unsafe {
      let id = ID;

      // Prevent panics on long-running debug builds
      ID = ID.wrapping_add(1);

      id
    }
  }

  pub(crate) fn stop_hydrating() {
    unsafe { IS_HYDRATING = false }
  }

  pub(crate) fn is_hydrating() -> bool {
    unsafe { IS_HYDRATING }
  }

  pub(crate) fn to_string(id: usize, closing: bool) -> String {
    #[cfg(debug_assertions)]
    return format!("_{id}{}", closing.then_some('c').unwrap_or('o'));

    #[cfg(not(debug_assertions))]
    return format!("_{id}");
  }
}
