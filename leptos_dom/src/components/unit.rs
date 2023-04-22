use cfg_if::cfg_if;
use std::fmt;

cfg_if! {
  if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
    use crate::Mountable;
    use wasm_bindgen::JsCast;
  } else {
    use crate::hydration::HydrationKey;
  }
}

use crate::{hydration::HydrationCtx, Comment, CoreComponent, IntoView, View};

/// The internal representation of the [`Unit`] core-component.
#[derive(Clone, PartialEq, Eq)]
pub struct UnitRepr {
    comment: Comment,
    #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
    pub(crate) id: HydrationKey,
}

impl fmt::Debug for UnitRepr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<() />")
    }
}

impl Default for UnitRepr {
    fn default() -> Self {
        let id = HydrationCtx::id();

        Self {
            comment: Comment::new("<() />", &id, true),
            #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
            id,
        }
    }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
impl Mountable for UnitRepr {
    #[inline(always)]
    fn get_mountable_node(&self) -> web_sys::Node {
        self.comment.node.clone().unchecked_into()
    }

    #[inline(always)]
    fn get_opening_node(&self) -> web_sys::Node {
        self.comment.node.clone().unchecked_into()
    }

    #[inline(always)]
    fn get_closing_node(&self) -> web_sys::Node {
        self.comment.node.clone().unchecked_into()
    }
}

/// The unit `()` leptos counterpart.
#[derive(Clone, Copy, Debug)]
pub struct Unit;

impl IntoView for Unit {
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "info", name = "<() />", skip_all)
    )]
    fn into_view(self, _: leptos_reactive::Scope) -> crate::View {
        let component = UnitRepr::default();

        View::CoreComponent(CoreComponent::Unit(component))
    }
}
