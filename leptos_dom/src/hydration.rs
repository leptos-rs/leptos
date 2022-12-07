use crate::TopoId;
use leptos_reactive::Scope;
use smallvec::{smallvec, SmallVec};
use std::{
  cell::{OnceCell, RefCell},
  rc::Rc,
};

type ParentOffsetSums = Vec<(usize, usize)>;

#[cfg(feature = "hydrate")]
const HYDRATION_MODE: bool = true;
#[cfg(not(feature = "hydrate"))]
const HYDRATION_MODE: bool = false;

#[thread_local]
static mut IS_HYDRATING: bool = HYDRATION_MODE;

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
    return format!("_{id}{}", if closing { 'c' } else { 'o' });

    #[cfg(not(debug_assertions))]
    return format!("_{id}");
  }
}
