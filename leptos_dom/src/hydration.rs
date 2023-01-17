use cfg_if::cfg_if;
use std::{cell::RefCell, fmt::Display};

cfg_if! {
  if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
    use once_cell::unsync::Lazy as LazyCell;
    use std::collections::HashMap;
    use wasm_bindgen::JsCast;

    // We can tell if we start in hydration mode by checking to see if the
    // id "_0-0-0" is present in the DOM. If it is, we know we are hydrating from
    // the server, if not, we are starting off in CSR
    thread_local! {
      static HYDRATION_COMMENTS: LazyCell<HashMap<String, web_sys::Comment>> = LazyCell::new(|| {
        let document = crate::document();
        let body = document.body().unwrap();
        let walker = document
          .create_tree_walker_with_what_to_show(&body, 128)
          .unwrap();
        let mut map = HashMap::new();
        while let Ok(Some(node)) = walker.next_node() {
          if let Some(content) = node.text_content() {
            if let Some(hk) = content.strip_prefix("hk=") {
              if let Some(hk) = hk.split("|").next() {
                map.insert(hk.into(), node.unchecked_into());
              }
            }
          }
        }
        map
      });

      static IS_HYDRATING: RefCell<LazyCell<bool>> = RefCell::new(LazyCell::new(|| {
        #[cfg(debug_assertions)]
        return crate::document().get_element_by_id("_0-0-0").is_some()
          || crate::document().get_element_by_id("_0-0-0o").is_some()
          || HYDRATION_COMMENTS.with(|comments| comments.get("_0-0-0o").is_some());

        #[cfg(not(debug_assertions))]
        return crate::document().get_element_by_id("_0-0-0").is_some()
          || HYDRATION_COMMENTS.with(|comments| comments.get("_0-0-0").is_some());
      }));
    }

    pub(crate) fn get_marker(id: &str) -> Option<web_sys::Comment> {
      HYDRATION_COMMENTS.with(|comments| comments.get(id).cloned())
    }
  }
}

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
    IS_HYDRATING.with(|is_hydrating| {
      std::mem::take(&mut *is_hydrating.borrow_mut());
    })
  }

  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  pub(crate) fn is_hydrating() -> bool {
    IS_HYDRATING.with(|is_hydrating| **is_hydrating.borrow())
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
