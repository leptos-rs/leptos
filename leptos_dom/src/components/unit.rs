use crate::{Comment, CoreComponent, IntoNode, Mountable, Node};
use wasm_bindgen::JsCast;

/// The internal representation of the [`Unit`] core-component.
#[derive(Debug)]
pub struct UnitRepr {
  comment: Comment,
}

impl Default for UnitRepr {
  fn default() -> Self {
    Self {
      comment: Comment::new("<() />"),
    }
  }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
impl Mountable for UnitRepr {
  fn get_mountable_node(&self) -> web_sys::Node {
    self.comment.node.clone().unchecked_into()
  }

  fn get_opening_node(&self) -> web_sys::Node {
    self.comment.node.clone().unchecked_into()
  }
}

/// The unit `()` leptos counterpart.
#[derive(Clone, Copy, Debug)]
pub struct Unit;

impl IntoNode for Unit {
  #[cfg_attr(
    debug_assertions,
    instrument(level = "trace", name = "<() />", skip_all)
  )]
  fn into_node(self, _: leptos_reactive::Scope) -> crate::Node {
    let component = UnitRepr::default();

    Node::CoreComponent(CoreComponent::Unit(component))
  }
}
