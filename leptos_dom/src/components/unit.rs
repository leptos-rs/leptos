use crate::{Comment, CoreComponent, IntoNode, Node};

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

impl UnitRepr {
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  pub(crate) fn get_web_sys_node(&self) -> web_sys::Node {
    use wasm_bindgen::JsCast;

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
