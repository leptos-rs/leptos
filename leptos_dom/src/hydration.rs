use leptos_reactive::Scope;

#[cfg(all(target_arch = "wasm32", feature = "web"))]
use std::cell::LazyCell;

/// We can tell if we start in hydration mode by checking to see if the
/// id "_0" is present in the DOM. If it is, we know we are hydrating from
/// the server, if not, we are starting off in CSR
#[cfg(all(target_arch = "wasm32", feature = "web"))]
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

#[thread_local]
static mut FORCE_HK: bool = false;

/// Control and utility methods for hydration.
pub struct HydrationCtx;

impl HydrationCtx {
  /// Get the next `id` without incrementing it.
  pub fn peak() -> usize {
    unsafe { ID }
  }

  /// Increments the current hydration `id` and returns it
  pub fn id() -> usize {
    unsafe {
      let id = ID;

      // Prevent panics on long-running debug builds
      ID = ID.wrapping_add(1);

      id
    }
  }

  pub(crate) fn current_id() -> usize {
    unsafe { ID }
  }

  #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
  pub(crate) fn reset_id() {
    unsafe { ID = 0 };
  }

  /// Resums hydration from the provided `id`. Usefull for
  /// `Suspense` and other fancy things.
  pub fn continue_from(id: usize) {
    unsafe { ID = id }
  }

  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  pub(crate) fn stop_hydrating() {
    unsafe {
      std::mem::take(&mut IS_HYDRATING);
    }
  }

  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  pub(crate) fn is_hydrating() -> bool {
    unsafe { *IS_HYDRATING }
  }

  pub(crate) fn to_string(id: usize, closing: bool) -> String {
    #[cfg(debug_assertions)]
    return format!("_{id}{}", if closing { 'c' } else { 'o' });

    #[cfg(not(debug_assertions))]
    {
      let _ = closing;

      format!("_{id}")
    }
  }

  /// All components and elements created after this method is
  /// called with use `leptos-hk` for their hydration `id`,
  /// instead of `id`.
  pub fn start_force_hk() {
    unsafe { FORCE_HK = true }
  }

  /// All components and elements created after this method is
  /// called with use `id` by default for their hydration `id`,
  /// instead of `leptos-hk`.
  pub fn stop_force_hk() {
    unsafe { FORCE_HK = false }
  }
}
