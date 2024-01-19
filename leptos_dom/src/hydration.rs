#[cfg(all(
    feature = "experimental-islands",
    any(feature = "hydrate", feature = "ssr")
))]
use leptos_reactive::SharedContext;
use std::{cell::RefCell, fmt::Display};

#[cfg(feature = "hydrate")]
mod hydrate_only {
    use once_cell::unsync::Lazy as LazyCell;
    use std::{cell::Cell, collections::HashMap};
    use wasm_bindgen::JsCast;

    /// See ["createTreeWalker"](https://developer.mozilla.org/en-US/docs/Web/API/Document/createTreeWalker)
    #[allow(unused)]
    const FILTER_SHOW_COMMENT: u32 = 0b10000000;

    thread_local! {
      pub static HYDRATION_COMMENTS: LazyCell<HashMap<String, web_sys::Comment>> = LazyCell::new(|| {
        let document = crate::document();
        let body = document.body().unwrap();
        let walker = document
          .create_tree_walker_with_what_to_show(&body, FILTER_SHOW_COMMENT)
          .unwrap();
        let mut map = HashMap::new();
        while let Ok(Some(node)) = walker.next_node() {
          if let Some(content) = node.text_content() {
            if let Some(hk) = content.strip_prefix("hk=") {
              if let Some(hk) = hk.split('|').next() {
                map.insert(hk.into(), node.unchecked_into());
              }
            }
          }
        }
        map
      });

      pub static HYDRATION_ELEMENTS: LazyCell<HashMap<String, web_sys::HtmlElement>> = LazyCell::new(|| {
        let document = crate::document();
        let els = document.query_selector_all("[data-hk]");
        if let Ok(list) = els {
            let len = list.length();
            let mut map = HashMap::with_capacity(len as usize);
            for idx in 0..len {
                let el = list.item(idx).unwrap().unchecked_into::<web_sys::HtmlElement>();
                let dataset = el.dataset();
                let hk = dataset.get(wasm_bindgen::intern("hk")).unwrap();
                map.insert(hk, el);
            }
            map
        } else {
            Default::default()
        }
      });

      pub static IS_HYDRATING: Cell<bool> = const { Cell::new(true) };
    }

    #[allow(unused)]
    pub fn get_marker(id: &str) -> Option<web_sys::Comment> {
        HYDRATION_COMMENTS.with(|comments| comments.get(id).cloned())
    }

    #[allow(unused)]
    pub fn get_element(hk: &str) -> Option<web_sys::HtmlElement> {
        HYDRATION_ELEMENTS.with(|els| els.get(hk).cloned())
    }
}

#[cfg(feature = "hydrate")]
pub(crate) use hydrate_only::*;

/// A stable identifier within the server-rendering or hydration process.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct HydrationKey {
    /// ID of the current outlet
    pub outlet: usize,
    /// ID of the current fragment.
    pub fragment: usize,
    /// ID of the current error boundary.
    pub error: usize,
    /// ID of the current key.
    pub id: usize,
}

impl Display for HydrationKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}-{}-{}-{}",
            self.outlet, self.fragment, self.error, self.id
        )
    }
}

impl std::str::FromStr for HydrationKey {
    type Err = (); // TODO better error

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut pieces = s.splitn(4, '-');
        let first = pieces.next().ok_or(())?;
        let second = pieces.next().ok_or(())?;
        let third = pieces.next().ok_or(())?;
        let fourth = pieces.next().ok_or(())?;
        let outlet = usize::from_str(first).map_err(|_| ())?;
        let fragment = usize::from_str(second).map_err(|_| ())?;
        let error = usize::from_str(third).map_err(|_| ())?;
        let id = usize::from_str(fourth).map_err(|_| ())?;
        Ok(HydrationKey {
            outlet,
            fragment,
            error,
            id,
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn parse_hydration_key() {
        use crate::HydrationKey;
        use std::str::FromStr;
        assert_eq!(
            HydrationKey::from_str("0-1-2-3"),
            Ok(HydrationKey {
                outlet: 0,
                fragment: 1,
                error: 2,
                id: 3
            })
        )
    }
}

thread_local!(static ID: RefCell<HydrationKey> = const {RefCell::new(HydrationKey { outlet: 0, fragment: 0, error: 0, id: 0 })});

/// Control and utility methods for hydration.
pub struct HydrationCtx;

impl HydrationCtx {
    /// If you're in an hydration context, get the next `id` without incrementing it.
    pub fn peek() -> Option<HydrationKey> {
        #[cfg(all(
            feature = "experimental-islands",
            any(feature = "hydrate", feature = "ssr")
        ))]
        let no_hydrate = SharedContext::no_hydrate();
        #[cfg(not(all(
            feature = "experimental-islands",
            any(feature = "hydrate", feature = "ssr")
        )))]
        let no_hydrate = false;
        if no_hydrate {
            None
        } else {
            Some(ID.with(|id| *id.borrow()))
        }
    }

    /// Get the next `id` without incrementing it.
    pub fn peek_always() -> HydrationKey {
        ID.with(|id| *id.borrow())
    }

    /// Increments the current hydration `id` and returns it
    pub fn id() -> Option<HydrationKey> {
        #[cfg(all(
            feature = "experimental-islands",
            any(feature = "hydrate", feature = "ssr")
        ))]
        let no_hydrate = SharedContext::no_hydrate();
        #[cfg(not(all(
            feature = "experimental-islands",
            any(feature = "hydrate", feature = "ssr")
        )))]
        let no_hydrate = false;

        if no_hydrate {
            None
        } else {
            Some(ID.with(|id| {
                let mut id = id.borrow_mut();
                id.id = id.id.wrapping_add(1);
                *id
            }))
        }
    }

    /// Resets the hydration `id` for the next component, and returns it
    pub fn next_component() -> HydrationKey {
        ID.with(|id| {
            let mut id = id.borrow_mut();
            id.fragment = id.fragment.wrapping_add(1);
            id.id = 0;
            *id
        })
    }

    /// Resets the hydration `id` for the next outlet, and returns it
    pub fn next_outlet() -> HydrationKey {
        ID.with(|id| {
            let mut id = id.borrow_mut();
            id.outlet = id.outlet.wrapping_add(1);
            id.id = 0;
            *id
        })
    }

    /// Resets the hydration `id` for the next component, and returns it
    pub fn next_error() -> HydrationKey {
        ID.with(|id| {
            let mut id = id.borrow_mut();
            id.error = id.error.wrapping_add(1);
            id.id = 0;
            *id
        })
    }

    #[doc(hidden)]
    #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
    pub fn reset_id() {
        ID.with(|id| {
            *id.borrow_mut() = HydrationKey {
                outlet: 0,
                fragment: 0,
                error: 0,
                id: 0,
            }
        });
    }

    /// Resumes hydration from the provided `id`. Useful for
    /// `Suspense` and other fancy things.
    pub fn continue_from(id: HydrationKey) {
        ID.with(|i| *i.borrow_mut() = id);
    }

    /// Resumes hydration after the provided `id`. Useful for
    /// islands and other fancy things.
    pub fn continue_after(id: HydrationKey) {
        ID.with(|i| {
            *i.borrow_mut() = HydrationKey {
                outlet: id.outlet,
                fragment: id.fragment,
                error: id.error,
                id: id.id + 1,
            }
        });
    }

    #[doc(hidden)]
    pub fn stop_hydrating() {
        #[cfg(feature = "hydrate")]
        {
            IS_HYDRATING.with(|is_hydrating| {
                is_hydrating.set(false);
            })
        }
    }

    #[doc(hidden)]
    #[cfg(feature = "hydrate")]
    pub fn with_hydration_on<T>(f: impl FnOnce() -> T) -> T {
        let prev = IS_HYDRATING.with(|is_hydrating| {
            let prev = is_hydrating.get();
            is_hydrating.set(true);
            prev
        });
        let value = f();
        IS_HYDRATING.with(|is_hydrating| is_hydrating.set(prev));
        value
    }

    #[doc(hidden)]
    #[cfg(feature = "hydrate")]
    pub fn with_hydration_off<T>(f: impl FnOnce() -> T) -> T {
        let prev = IS_HYDRATING.with(|is_hydrating| {
            let prev = is_hydrating.get();
            is_hydrating.set(false);
            prev
        });
        let value = f();
        IS_HYDRATING.with(|is_hydrating| is_hydrating.set(prev));
        value
    }

    /// Whether the UI is currently in the process of hydrating from the server-sent HTML.
    #[inline(always)]
    pub fn is_hydrating() -> bool {
        #[cfg(feature = "hydrate")]
        {
            IS_HYDRATING.with(|is_hydrating| is_hydrating.get())
        }
        #[cfg(not(feature = "hydrate"))]
        {
            false
        }
    }

    #[cfg(feature = "hydrate")]
    #[allow(unused)]
    pub(crate) fn to_string(id: &HydrationKey, closing: bool) -> String {
        #[cfg(debug_assertions)]
        return format!("{id}{}", if closing { 'c' } else { 'o' });

        #[cfg(not(debug_assertions))]
        {
            id.to_string()
        }
    }
}
