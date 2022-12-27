use std::{cell::RefCell, fmt::Display};

#[cfg(all(target_arch = "wasm32", feature = "web"))]
use std::cell::LazyCell;

/// We can tell if we start in hydration mode by checking to see if the
/// id "_0" is present in the DOM. If it is, we know we are hydrating from
/// the server, if not, we are starting off in CSR
#[cfg(all(target_arch = "wasm32", feature = "web"))]
#[thread_local]
static mut IS_HYDRATING: LazyCell<bool> = LazyCell::new(|| {
  #[cfg(debug_assertions)]
  return crate::document().get_element_by_id("_0-0-0").is_some()
    || crate::document().get_element_by_id("_0-0-0o").is_some();

  #[cfg(not(debug_assertions))]
  return crate::document().get_element_by_id("_0-0-0").is_some();
});

/// A stable identifer within the server-rendering or hydration process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HydrationKey {
  /// The key of the previous component.
  pub previous: String,
  /// The element offset within the current component.
  pub offset: usize,
}

impl Display for HydrationKey {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}{}", self.previous, self.offset)
  }
}

impl Default for HydrationKey {
  fn default() -> Self {
    Self {
      previous: "0-".to_string(),
      offset: 0,
    }
  }
}

thread_local!(static ID: RefCell<HydrationKey> = Default::default());

/// Control and utility methods for hydration.
pub struct HydrationCtx;

impl HydrationCtx {
  /// Get the next `id` without incrementing it.
  pub fn peek() -> HydrationKey {
    ID.with(|id| id.borrow().clone())
  }

  /// Increments the current hydration `id` and returns it
  pub fn id() -> HydrationKey {
    ID.with(|id| {
      let mut id = id.borrow_mut();
      id.offset = id.offset.wrapping_add(1);
      id.clone()
    })
  }

  /// Resets the hydration `id` for the next component, and returns it
  pub fn next_component() -> HydrationKey {
    ID.with(|id| {
      let mut id = id.borrow_mut();
      let offset = id.offset;
      id.previous.push_str(&offset.to_string());
      id.previous.push('-');
      id.offset = 0;
      id.clone()
    })
  }

  #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
  pub(crate) fn reset_id() {
    ID.with(|id| *id.borrow_mut() = Default::default());
  }

  /// Resums hydration from the provided `id`. Usefull for
  /// `Suspense` and other fancy things.
  pub fn continue_from(id: HydrationKey) {
    ID.with(|i| *i.borrow_mut() = id);
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

  pub(crate) fn to_string(id: &HydrationKey, closing: bool) -> String {
    #[cfg(debug_assertions)]
    return format!("_{id}{}", if closing { 'c' } else { 'o' });

    #[cfg(not(debug_assertions))]
    {
      let _ = closing;

      format!("_{id}")
    }
  }
}
