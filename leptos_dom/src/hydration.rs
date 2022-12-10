use std::cell::LazyCell;

/// We can tell if we start in hydration mode by checking to see if the
/// id "_0" is present in the DOM. If it is, we know we are hydrating from
/// the server, if not, we are starting off in CSR
#[thread_local]
static mut IS_HYDRATING: LazyCell<bool> = LazyCell::new(|| {
  #[cfg(debug_assertions)]
  return crate::document().get_element_by_id("_0").is_some()
    || crate::document().get_element_by_id("_0o").is_some();

  #[cfg(not(debug_assertions))]
  return crate::document().get_element_by_id("_0").is_some();
});

#[thread_local]
static mut ID: usize = 0;

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

  #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
  pub(crate) fn reset_id() {
    unsafe { ID = 0 };
  }

  pub(crate) fn stop_hydrating() {
    unsafe {
      std::mem::take(&mut IS_HYDRATING);
    }
  }

  pub(crate) fn is_hydrating() -> bool {
    unsafe { *IS_HYDRATING }
  }

  pub(crate) fn to_string(id: usize, closing: bool) -> String {
    #[cfg(debug_assertions)]
    return format!("_{id}{}", if closing { 'c' } else { 'o' });

    #[cfg(not(debug_assertions))]
    return format!("_{id}");
  }
}
