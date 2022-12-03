#![deny(missing_docs)]
#![feature(once_cell, iter_intersperse, drain_filter, thread_local)]
#![cfg_attr(not(feature = "stable"), feature(fn_traits))]
#![cfg_attr(not(feature = "stable"), feature(unboxed_closures))]

//! The DOM implementation for `leptos`.

#[macro_use]
extern crate clone_macro;
#[macro_use]
extern crate tracing;

mod components;
mod events;
mod html;
mod logging;
mod macro_helpers;
mod node_ref;

use cfg_if::cfg_if;
pub use components::*;
pub use html::*;
use leptos_reactive::Scope;
pub use logging::*;
pub use node_ref::*;
use smallvec::SmallVec;
use std::{
  borrow::Cow,
  cell::{LazyCell, OnceCell},
  fmt,
};
use wasm_bindgen::{JsCast, UnwrapThrowExt};

#[thread_local]
static COMMENT: LazyCell<web_sys::Node> =
  LazyCell::new(|| document().create_comment("").unchecked_into());
#[cfg(all(target_arch = "wasm32", feature = "web"))]
#[thread_local]
static RANGE: LazyCell<web_sys::Range> =
  LazyCell::new(|| web_sys::Range::new().unwrap());

/// Converts the value into a [`Node`].
pub trait IntoNode {
  /// Converts the value into [`Node`].
  fn into_node(self, cx: Scope) -> Node;
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
trait Mountable {
  /// Gets the [`web_sys::Node`] that can be directly inserted as
  /// a child of another node. Typically, this is a [`web_sys::DocumentFragment`]
  /// for components, and [`web_sys::HtmlElement`] for elements.
  fn get_mountable_node(&self) -> web_sys::Node;

  /// Get's the first node of the [`Node`].
  /// Typically, for [`HtmlElement`], this will be the
  /// `element` node. For components, this would be the
  /// first child node, or the `closing` marker comment node if
  /// no children are available.
  fn get_opening_node(&self) -> web_sys::Node;
}

impl IntoNode for () {
  #[cfg_attr(
    debug_assertions,
    instrument(level = "trace", name = "<() />", skip_all)
  )]
  fn into_node(self, cx: Scope) -> Node {
    Unit.into_node(cx)
  }
}

impl<T> IntoNode for Option<T>
where
  T: IntoNode,
{
  #[cfg_attr(
    debug_assertions,
    instrument(level = "trace", name = "Option<T>", skip_all)
  )]
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
  #[cfg_attr(
    debug_assertions,
    instrument(level = "trace", name = "Fn() -> N", skip_all)
  )]
  fn into_node(self, cx: Scope) -> Node {
    DynChild::new(self).into_node(cx)
  }
}

cfg_if! {
  if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
    /// HTML element.
    #[derive(Debug)]
    pub struct Element {
      #[cfg(debug_assertions)]
      name: Cow<'static, str>,
      element: web_sys::HtmlElement,
    }
  } else {
    /// HTML element.
    #[derive(Debug)]
    pub struct Element {
      name: Cow<'static, str>,
      is_void: bool,
      attrs: SmallVec<[(Cow<'static, str>, Cow<'static, str>); 4]>,
      children: Vec<Node>,
    }
  }
}

impl IntoNode for Element {
  #[cfg_attr(debug_assertions, instrument(level = "trace", name = "<Element />", skip_all, fields(tag = %self.name)))]
  fn into_node(self, _: Scope) -> Node {
    Node::Element(self)
  }
}

impl Element {
  #[track_caller]
  fn new<El: IntoElement>(el: El) -> Self {
    cfg_if! {
      if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
          Self {
            #[cfg(debug_assertions)]
            name: el.name(),
            element: el.get_element().clone(),
          }
      }
      else {
        Self {
          name: el.name(),
          is_void: el.is_void(),
          attrs: Default::default(),
          children: Default::default(),
        }
      }
    }
  }
}

#[derive(Debug)]
struct Comment {
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  node: web_sys::Node,
  content: Cow<'static, str>,
}

impl Comment {
  fn new(content: impl Into<Cow<'static, str>>) -> Self {
    let content = content.into();

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    let node = COMMENT.clone_node().unwrap();

    #[cfg(all(debug_assertions, target_arch = "wasm32", feature = "web"))]
    node.set_text_content(Some(&format!(" {content} ")));

    Self {
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      node,
      content,
    }
  }
}

/// HTML text
#[derive(Debug)]
pub struct Text {
  /// In order to support partial updates on text nodes, that is,
  /// to update the node without recreating it, we need to be able
  /// to possibly reuse a previous node.
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  node: web_sys::Node,
  content: Cow<'static, str>,
}

impl IntoNode for Text {
  #[cfg_attr(debug_assertions, instrument(level = "trace", name = "#text", skip_all, fields(content = %self.content)))]
  fn into_node(self, _: Scope) -> Node {
    Node::Text(self)
  }
}

impl Text {
  /// Creates a new [`Text`].
  pub fn new(content: impl Into<Cow<'static, str>>) -> Self {
    let content = content.into();

    Self {
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      node: crate::document()
        .create_text_node(&content)
        .unchecked_into::<web_sys::Node>(),
      content,
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
  Component(ComponentRepr),
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
  #[cfg_attr(debug_assertions, instrument(level = "trace", name = "Node", skip_all, fields(kind = self.kind_name())))]
  fn into_node(self, _: Scope) -> Node {
    self
  }
}

impl IntoNode for Vec<Node> {
  #[cfg_attr(
    debug_assertions,
    instrument(level = "trace", name = "Vec<Node>", skip_all)
  )]
  fn into_node(self, cx: Scope) -> Node {
    Fragment::new(self).into_node(cx)
  }
}

impl<const N: usize> IntoNode for [Node; N] {
  #[cfg_attr(
    debug_assertions,
    instrument(level = "trace", name = "[Node; N]", skip_all)
  )]
  fn into_node(self, cx: Scope) -> Node {
    Fragment::new(self.into_iter().collect()).into_node(cx)
  }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
impl Mountable for Node {
  fn get_mountable_node(&self) -> web_sys::Node {
    match self {
      Self::Element(element) => {
        element.element.unchecked_ref::<web_sys::Node>().clone()
      }
      Self::Text(t) => t.node.clone(),
      Self::CoreComponent(c) => match c {
        CoreComponent::Unit(u) => u.get_mountable_node(),
        CoreComponent::DynChild(dc) => dc.get_mountable_node(),
        CoreComponent::Each(e) => e.get_mountable_node(),
      },
      Self::Component(c) => c.get_mountable_node(),
    }
  }

  fn get_opening_node(&self) -> web_sys::Node {
    match self {
      Self::Text(t) => t.node.clone(),
      Self::Element(el) => el.element.clone().unchecked_into(),
      Self::CoreComponent(c) => match c {
        CoreComponent::DynChild(dc) => todo!(),
        CoreComponent::Each(e) => e.get_opening_node(),
        CoreComponent::Unit(u) => u.get_opening_node(),
      },
      Self::Component(c) => c.get_opening_node(),
    }
  }
}

impl Node {
  fn kind_name(&self) -> &'static str {
    match self {
      Self::Component(..) => "Component",
      Self::Element(..) => "Element",
      Self::Text(..) => "Text",
      Self::CoreComponent(c) => match c {
        CoreComponent::DynChild(..) => "DynChild",
        CoreComponent::Each(..) => "Each",
        CoreComponent::Unit(..) => "Unit",
      },
    }
  }

  fn get_text(&self) -> Option<&Text> {
    if let Self::Text(t) = self {
      Some(t)
    } else {
      None
    }
  }
}

#[cfg_attr(debug_assertions, instrument)]
#[track_caller]
#[cfg(all(target_arch = "wasm32", feature = "web"))]
fn mount_child<GWSN: Mountable + fmt::Debug>(kind: MountKind, child: &GWSN) {
  let child = child.get_mountable_node();

  match kind {
    MountKind::Append(el) => {
      el.append_child(&child)
        .expect("append operation to not err");
    }
    MountKind::Before(closing) => {
      closing
        .unchecked_ref::<web_sys::Element>()
        .before_with_node_1(&child)
        .expect("before to not err");
    }
    MountKind::After(closing) => {
      closing
        .unchecked_ref::<web_sys::Element>()
        .after_with_node_1(&child)
        .expect("before to not err");
    }
  }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
#[derive(Debug)]
enum MountKind<'a> {
  Before(
    // The closing node
    &'a web_sys::Node,
  ),
  Append(&'a web_sys::Node),
  After(
    // The opening node
    &'a web_sys::Node,
  ),
}

/// Runs the provided closure and mounts the result to eht `<body>`.
#[cfg(all(target_arch = "wasm32", feature = "web"))]
pub fn mount_to_body<F, N>(f: F)
where
  F: FnOnce(Scope) -> N + 'static,
  N: IntoNode,
{
  mount_to(crate::document().body().expect("body element to exist"), f)
}

/// Runs the provided closure and mounts the result to the provided element.
#[cfg(all(target_arch = "wasm32", feature = "web"))]
pub fn mount_to<F, N>(parent: web_sys::HtmlElement, f: F)
where
  F: FnOnce(Scope) -> N + 'static,
  N: IntoNode,
{
  let disposer = leptos_reactive::create_scope(
    leptos_reactive::create_runtime(),
    move |cx| {
      let node = f(cx).into_node(cx);

      parent.append_child(&node.get_mountable_node()).unwrap();

      std::mem::forget(node);
    },
  );

  std::mem::forget(disposer);
}

thread_local! {
    pub(crate) static WINDOW: web_sys::Window = web_sys::window().unwrap_throw();

    pub(crate) static DOCUMENT: web_sys::Document = web_sys::window().unwrap_throw().document().unwrap_throw();
}

/// Returns the [`Window`](https://developer.mozilla.org/en-US/docs/Web/API/Window).
///
/// This is cached as a thread-local variable, so calling `window()` multiple times
/// requires only one call out to JavaScript.
pub fn window() -> web_sys::Window {
  WINDOW.with(|window| window.clone())
}

/// Returns the [`Document`](https://developer.mozilla.org/en-US/docs/Web/API/Document).
///
/// This is cached as a thread-local variable, so calling `window()` multiple times
/// requires only one call out to JavaScript.
pub fn document() -> web_sys::Document {
  DOCUMENT.with(|document| document.clone())
}

/// Shorthand to test for whether an `ssr` feature is enabled.
///
/// In the past, this was implemented by checking whether `not(target_arch = "wasm32")`.
/// Now that some cloud platforms are moving to run Wasm on the edge, we really can't
/// guarantee that compiling to Wasm means browser APIs are available, or that not compiling
/// to Wasm means we're running on the server.
///
/// ```
/// # use leptos_dom::is_server;
/// let todos = if is_server!() {
///   // if on the server, load from DB
/// } else {
///   // if on the browser, do something else
/// };
/// ```
#[macro_export]
macro_rules! is_server {
  () => {
    cfg!(feature = "ssr")
  };
}

/// A shorthand macro to test whether this is a debug build.
/// ```
/// # use leptos_dom::is_dev;
/// if is_dev!() {
///   // log something or whatever
/// }
/// ```
#[macro_export]
macro_rules! is_dev {
  () => {
    cfg!(debug_assertions)
  };
}
