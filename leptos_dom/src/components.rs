mod dyn_child;
mod each;
mod fragment;
mod unit;

#[cfg(all(target_arch = "wasm32", feature = "web"))]
use crate::{mount_child, MountKind, Mountable};
use crate::{Comment, IntoNode, Node};
pub use dyn_child::*;
pub use each::*;
pub use fragment::*;
use leptos_reactive::{Scope, ScopeDisposer};
use std::borrow::Cow;
pub use unit::*;
use wasm_bindgen::JsCast;

/// The core foundational leptos components.
#[derive(Debug, educe::Educe)]
#[educe(Default)]
pub enum CoreComponent {
  /// The [`Unit`] component.
  #[educe(Default)]
  Unit(UnitRepr),
  /// The [`DynChild`] component.
  DynChild(DynChildRepr),
  /// The [`Each`] component.
  Each(EachRepr),
}

/// Custom leptos component.
#[derive(Debug)]
pub struct ComponentRepr {
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  document_fragment: web_sys::DocumentFragment,
  #[cfg(debug_assertions)]
  name: Cow<'static, str>,
  #[cfg(debug_assertions)]
  _opening: Comment,
  /// The children of the component.
  pub children: Vec<Node>,
  closing: Comment,
  disposer: Option<ScopeDisposer>,
}

impl Drop for ComponentRepr {
  fn drop(&mut self) {
    if let Some(disposer) = self.disposer.take() {
      disposer.dispose();
    }
  }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
impl Mountable for ComponentRepr {
  fn get_mountable_node(&self) -> web_sys::Node {
    self
      .document_fragment
      .unchecked_ref::<web_sys::Node>()
      .clone()
  }

  fn get_opening_node(&self) -> web_sys::Node {
    if let Some(child) = self.children.get(0) {
      child.get_opening_node()
    } else {
      self.closing.node.clone()
    }
  }
}

impl IntoNode for ComponentRepr {
  #[cfg_attr(debug_assertions, instrument(level = "trace", name = "<Component />", skip_all, fields(name = %self.name)))]
  fn into_node(self, _: Scope) -> Node {
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    for child in &self.children {
      mount_child(MountKind::Before(&self.closing.node), child);
    }

    Node::Component(self)
  }
}

impl ComponentRepr {
  /// Creates a new [`Component`].
  pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
    let name = name.into();

    let markers = (
      Comment::new(Cow::Owned(format!("</{name}>"))),
      #[cfg(debug_assertions)]
      Comment::new(Cow::Owned(format!("<{name}>"))),
    );

    #[cfg(not(debug_assertions))]
    let closing = Comment::new("");

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    let document_fragment = {
      let fragment = crate::document().create_document_fragment();

      // Insert the comments into the document fragment
      // so they can serve as our references when inserting
      // future nodes
      #[cfg(debug_assertions)]
      fragment
        .append_with_node_2(&markers.1.node, &markers.0.node)
        .expect("append to not err");
      #[cfg(not(debug_assertions))]
      fragment
        .append_with_node_1(&markers.0.node)
        .expect("append to not err");

      fragment
    };

    Self {
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      document_fragment,
      #[cfg(debug_assertions)]
      _opening: markers.1,
      closing: markers.0,
      #[cfg(debug_assertions)]
      name,
      children: Default::default(),
      disposer: Default::default(),
    }
  }
}

/// A user-defined `leptos` component.
pub struct Component<F>
where
  F: FnOnce(Scope) -> Vec<Node>,
{
  name: Cow<'static, str>,
  children_fn: F,
}

impl<F> Component<F>
where
  F: FnOnce(Scope) -> Vec<Node>,
{
  /// Creates a new component.
  pub fn new(name: impl Into<Cow<'static, str>>, f: F) -> Self {
    Self {
      name: name.into(),
      children_fn: f,
    }
  }
}

impl<F> IntoNode for Component<F>
where
  F: FnOnce(Scope) -> Vec<Node>,
{
  fn into_node(self, cx: Scope) -> Node {
    let Self { name, children_fn } = self;

    let mut children = None;

    let disposer = cx.child_scope(|cx| children = Some(children_fn(cx)));

    let children = children.unwrap();

    let mut repr = ComponentRepr::new(name);

    repr.disposer = Some(disposer);
    repr.children = children;

    repr.into_node(cx)
  }
}
