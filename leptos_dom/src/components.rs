mod dyn_child;
mod each;
mod fragment;
mod unit;

use crate::{hydration::HydrationCtx, Comment, IntoView, View};
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use crate::{mount_child, MountKind, Mountable};
pub use dyn_child::*;
pub use each::*;
pub use fragment::*;
use leptos_reactive::{Scope, ScopeDisposer};
use std::borrow::Cow;
pub use unit::*;
use wasm_bindgen::JsCast;

/// The core foundational leptos components.
#[derive(Debug, educe::Educe)]
#[educe(Default, Clone, PartialEq, Eq)]
pub enum CoreComponent {
  /// The [`Unit`] component.
  #[educe(Default)]
  Unit(UnitRepr),
  /// The [`DynChild`] component.
  DynChild(DynChildRepr),
  /// The [`EachKey`] component.
  Each(EachRepr),
}

/// Custom leptos component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentRepr {
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  pub(crate) document_fragment: web_sys::DocumentFragment,
  #[cfg(debug_assertions)]
  name: Cow<'static, str>,
  #[cfg(debug_assertions)]
  _opening: Comment,
  /// The children of the component.
  pub children: Vec<View>,
  closing: Comment,
  #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
  pub(crate) id: usize,
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
impl Mountable for ComponentRepr {
  fn get_mountable_node(&self) -> web_sys::Node {
    self
      .document_fragment
      .unchecked_ref::<web_sys::Node>()
      .to_owned()
  }

  fn get_opening_node(&self) -> web_sys::Node {
    #[cfg(debug_assertions)]
    return self._opening.node.clone();

    #[cfg(not(debug_assertions))]
    return if let Some(child) = self.children.get(0) {
      child.get_opening_node()
    } else {
      self.closing.node.clone()
    };
  }
}

impl IntoView for ComponentRepr {
  #[cfg_attr(debug_assertions, instrument(level = "trace", name = "<Component />", skip_all, fields(name = %self.name)))]
  fn into_view(self, _: Scope) -> View {
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    if !HydrationCtx::is_hydrating() {
      for child in &self.children {
        mount_child(MountKind::Before(&self.closing.node), child);
      }
    }

    View::Component(self)
  }
}

impl ComponentRepr {
  /// Creates a new [`Component`].
  pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
    let name = name.into();

    let id = HydrationCtx::id();

    let markers = (
      Comment::new(Cow::Owned(format!("</{name}>")), id, true),
      #[cfg(debug_assertions)]
      Comment::new(Cow::Owned(format!("<{name}>")), id, false),
    );

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    let document_fragment = {
      let fragment = crate::document().create_document_fragment();

      // Insert the comments into the document fragment
      // so they can serve as our references when inserting
      // future nodes
      if !HydrationCtx::is_hydrating() {
        #[cfg(debug_assertions)]
        fragment
          .append_with_node_2(&markers.1.node, &markers.0.node)
          .expect("append to not err");
        #[cfg(not(debug_assertions))]
        fragment
          .append_with_node_1(&markers.0.node)
          .expect("append to not err");
      }

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
      children: Vec::with_capacity(1),
      #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
      id,
    }
  }
}

/// A user-defined `leptos` component.
pub struct Component<F>
where
  F: FnOnce(Scope) -> View,
{
  name: Cow<'static, str>,
  children_fn: F,
}

impl<F> Component<F>
where
  F: FnOnce(Scope) -> View,
{
  /// Creates a new component.
  pub fn new(name: impl Into<Cow<'static, str>>, f: F) -> Self {
    Self {
      name: name.into(),
      children_fn: f,
    }
  }
}

impl<F> IntoView for Component<F>
where
  F: FnOnce(Scope) -> View,
{
  fn into_view(self, cx: Scope) -> View {
    let Self { name, children_fn } = self;

    let mut repr = ComponentRepr::new(name);

    let (child, disposer) =
      cx.run_child_scope(|cx| cx.untrack(|| children_fn(cx)));

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    leptos_reactive::on_cleanup(cx, || disposer.dispose());

    repr.children.push(child);

    repr.into_view(cx)
  }
}
