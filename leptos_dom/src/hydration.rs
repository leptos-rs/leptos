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

      ID += 1;

      id
    }
  }

  pub(crate) fn stop_hydrating() {
    unsafe { IS_HYDRATING = false }
  }

  pub(crate) fn is_hydrating() -> bool {
    unsafe { IS_HYDRATING }
  }

  // pub(crate) fn tick_child() {
  //   unsafe { PARENT_OFFSET_SUMS.push((ID.offset, ID.sum)) }

  //   unsafe {
  //     ID.sum += ID.depth + ID.offset;
  //     ID.depth += 1;
  //     ID.offset = 0;
  //   }
  // }

  // pub(crate) fn tick_sibling() {
  //   unsafe { ID.offset += 1 }
  // }

  // pub(crate) fn tick_parent() {
  //   let (parent_offset, parent_sum) =
  //     unsafe { PARENT_OFFSET_SUMS.pop().unwrap() };

  //   unsafe {
  //     ID.depth -= 1;
  //     ID.offset = parent_offset;
  //     ID.sum = parent_sum;
  //   }
  // }
}
