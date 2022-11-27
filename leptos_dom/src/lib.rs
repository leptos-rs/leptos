#![deny(missing_docs)]
#![feature(once_cell)]

//! The DOM implementation for `leptos`.

#[macro_use]
extern crate clone_macro;
#[macro_use]
extern crate tracing;

mod components;
mod html;

pub use components::*;
pub use html::*;
use leptos_reactive::Scope;
use smallvec::SmallVec;
use std::{borrow::Cow, fmt};
use wasm_bindgen::JsCast;

/// Converts the value into a [`Node`].
pub trait IntoNode {
  /// Converts the value into [`Node`].
  fn into_node(self, cx: Scope) -> Node;
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
trait GetWebSysNode {
  fn get_web_sys_node(&self) -> web_sys::Node;
}

impl IntoNode for () {
  #[instrument(level = "trace")]
  fn into_node(self, cx: Scope) -> Node {
    Unit.into_node(cx)
  }
}

impl<T> IntoNode for Option<T>
where
  T: IntoNode,
{
  #[instrument(level = "trace", skip_all)]
  fn into_node(self, cx: Scope) -> Node {
    if let Some(t) = self {
      t.into_node(cx)
    } else {
      Unit.into_node(cx)
    }
  }
}

impl<F, N> IntoNode for F
where
  F: Fn() -> N + 'static,
  N: IntoNode,
{
  #[instrument(level = "trace", skip_all)]
  fn into_node(self, cx: Scope) -> Node {
    DynChild::new(self).into_node(cx)
  }
}

cfg_if::cfg_if! {
    if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
        #[derive(Debug, educe::Educe)]
        #[educe(Deref)]
        // Be careful not to drop this until you want to unmount
        // the node from the DOM.
        struct WebSysNode(web_sys::Node);

        impl Drop for WebSysNode {
            #[instrument(level = "trace")]
            fn drop(&mut self) {
                let text_content = self.0.text_content();

                tracing::debug!(text_content, "dropping node");

                self.0.unchecked_ref::<web_sys::Element>().remove();
            }
        }

        impl From<web_sys::Node> for WebSysNode {
            fn from(node: web_sys::Node) -> Self {
                Self(node)
            }
        }
    } else {
        #[derive(Debug)]
        struct WebSysNode();
    }
}

/// HTML element.
#[derive(Debug)]
pub struct Element {
  _name: Cow<'static, str>,
  is_void: bool,
  node: WebSysNode,
  attrs: SmallVec<[(Cow<'static, str>, Cow<'static, str>); 4]>,
  children: Vec<Node>,
}

impl IntoNode for Element {
  #[instrument(level = "trace")]
  fn into_node(self, _: Scope) -> Node {
    Node::Element(self)
  }
}

impl Element {
  #[track_caller]
  fn new<El: IntoElement>(el: El) -> Self {
    let name = el.name();

    let node = 'label: {
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      break 'label gloo::utils::document()
        .create_element(&name)
        .expect("element creation to not fail")
        .unchecked_into::<web_sys::Node>()
        .into();

      #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
      break 'label WebSysNode();
    };

    Self {
      _name: name,
      is_void: el.is_void(),
      node,
      attrs: Default::default(),
      children: Default::default(),
    }
  }
}

#[derive(Debug)]
struct Comment {
  node: WebSysNode,
  content: Cow<'static, str>,
}

impl Comment {
  fn new(content: impl Into<Cow<'static, str>>) -> Self {
    let content = content.into();

    let node = 'label: {
      #[cfg(all(debug_assertions, target_arch = "wasm32", feature = "web"))]
      break 'label gloo::utils::document()
        .create_comment(&format!(" {content} "))
        .unchecked_into::<web_sys::Node>()
        .into();

      #[cfg(all(
        not(debug_assertions),
        target_arch = "wasm32",
        feature = "web"
      ))]
      break 'label gloo::utils::document()
        .create_comment("")
        .unchecked_into::<web_sys::Node>()
        .into();

      #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
      break 'label WebSysNode();
    };

    Self { node, content }
  }
}

/// HTML text
#[derive(Debug)]
pub struct Text {
  node: WebSysNode,
  content: Cow<'static, str>,
}

impl IntoNode for Text {
  #[instrument(level = "trace")]
  fn into_node(self, _: Scope) -> Node {
    Node::Text(self)
  }
}

impl Text {
  /// Creates a new [`Text`].
  pub fn new(content: impl Into<Cow<'static, str>>) -> Self {
    let content = content.into();

    let node = 'label: {
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      break 'label gloo::utils::document()
        .create_text_node(&content)
        .unchecked_into::<web_sys::Node>()
        .into();

      #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
      break 'label WebSysNode();
    };

    Self { content, node }
  }
}

/// Custom leptos component.
#[derive(Debug)]
pub struct Component {
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  document_fragment: web_sys::DocumentFragment,
  #[cfg(debug_assertions)]
  name: Cow<'static, str>,
  #[cfg(debug_assertions)]
  opening: Comment,
  /// The children of the component.
  pub children: Vec<Node>,
  closing: Comment,
}

impl IntoNode for Component {
  #[instrument(level = "trace")]
  fn into_node(self, _: Scope) -> Node {
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    for child in &self.children {
      mount_child(MountKind::Component(&self.closing.node), child);
    }

    Node::Component(self)
  }
}

impl Component {
  /// Creates a new [`Component`].
  pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
    let name = name.into();

    #[cfg(debug_assertions)]
    let (opening, closing) = {
      let opening = Comment::new(Cow::Owned(format!("<{name}>")));
      let closing = Comment::new(Cow::Owned(format!("</{name}>")));

      (opening, closing)
    };
    #[cfg(not(debug_assertions))]
    let closing = Comment::new("");

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    let document_fragment = {
      let fragment = gloo::utils::document().create_document_fragment();

      // Insert the comments into the document fragment
      // so they can serve as our references when inserting
      // future nodes
      #[cfg(debug_assertions)]
      fragment
        .append_with_node_2(&opening.node, &closing.node)
        .expect("append to not err");

      #[cfg(not(debug_assertions))]
      fragment
        .append_with_node_1(&closing.node)
        .expect("append to not err");

      fragment
    };

    Self {
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      document_fragment,
      #[cfg(debug_assertions)]
      opening,
      closing,
      #[cfg(debug_assertions)]
      name,
      children: Default::default(),
    }
  }
}

/// A leptos Node.
#[derive(Debug)]
pub enum Node {
  /// HTML element node.
  Element(Element),
  /// HTML text node.
  Text(Text),
  /// Custom leptos component.
  Component(Component),
  /// leptos core-component.
  CoreComponent(CoreComponent),
}

/// The default [`Node`] is the [`Unit`] core-component.
impl Default for Node {
  fn default() -> Self {
    Self::CoreComponent(Default::default())
  }
}

impl IntoNode for Node {
  #[instrument(level = "trace")]
  fn into_node(self, _: Scope) -> Node {
    self
  }
}

impl IntoNode for Vec<Node> {
  #[instrument(level = "trace")]
  fn into_node(self, cx: Scope) -> Node {
    Fragment::new(self).into_node(cx)
  }
}

impl<const N: usize> IntoNode for [Node; N] {
  #[instrument(level = "trace")]
  fn into_node(self, cx: Scope) -> Node {
    Fragment::new(self.into_iter().collect()).into_node(cx)
  }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
impl GetWebSysNode for Node {
  fn get_web_sys_node(&self) -> web_sys::Node {
    match self {
      Self::Element(node) => node.node.0.clone(),
      Self::Text(t) => t.node.0.clone(),
      Self::CoreComponent(c) => match c {
        CoreComponent::Unit(u) => u.get_web_sys_node(),
        CoreComponent::DynChild(dc) => dc.get_web_sys_node(),
        CoreComponent::Each(e) => e.get_web_sys_node(),
      },
      Self::Component(c) => c.document_fragment.clone().unchecked_into(),
    }
  }
}

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

#[instrument]
#[track_caller]
#[cfg(all(target_arch = "wasm32", feature = "web"))]
fn mount_child<GWSN: GetWebSysNode + fmt::Debug>(
  kind: MountKind,
  child: &GWSN,
) {
  let child = child.get_web_sys_node();

  match kind {
    MountKind::Component(closing) => {
      closing
        .unchecked_ref::<web_sys::Element>()
        .before_with_node_1(&child)
        .expect("before to not err");
    }
    MountKind::Element(el) => {
      el.append_child(&child)
        .expect("append operation to not err");
    }
  }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
#[derive(Debug)]
enum MountKind<'a> {
  Component(
    // The closing node
    &'a web_sys::Node,
  ),
  Element(&'a web_sys::Node),
}

/// Runs the provided closure and mounts the result to eht `<body>`.
#[cfg(all(target_arch = "wasm32", feature = "web"))]
pub fn mount_to_body<F, N>(f: F)
where
  F: FnOnce(Scope) -> N + 'static,
  N: IntoNode,
{
  let disposer =
    leptos_reactive::create_scope(leptos_reactive::create_runtime(), |cx| {
      let root = gloo::utils::document()
        .body()
        .expect("body element to exist");

      let node = f(cx).into_node(cx);

      root.append_child(&node.get_web_sys_node()).unwrap();

      std::mem::forget(node);
    });

  std::mem::forget(disposer);
}
