#![deny(missing_docs)]
#![feature(once_cell, iter_intersperse, drain_filter, thread_local)]
#![cfg_attr(not(feature = "stable"), feature(fn_traits))]
#![cfg_attr(not(feature = "stable"), feature(unboxed_closures))]

//! The DOM implementation for `leptos`.

#[cfg_attr(debug_assertions, macro_use)]
pub extern crate tracing;

mod components;
mod events;
mod helpers;
mod html;
mod hydration;
mod logging;
mod macro_helpers;
mod node_ref;
mod ssr;
mod transparent;

use cfg_if::cfg_if;
pub use components::*;
pub use events::typed as ev;
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use events::{add_event_listener, add_event_listener_undelegated};
pub use helpers::*;
pub use html::*;
pub use hydration::{HydrationCtx, HydrationKey};
pub use js_sys;
use leptos_reactive::Scope;
pub use logging::*;
pub use macro_helpers::{IntoAttribute, IntoClass, IntoProperty};
pub use node_ref::*;
#[cfg(not(all(target_arch = "wasm32", feature = "web")))]
use smallvec::SmallVec;
#[cfg(not(all(target_arch = "wasm32", feature = "web")))]
pub use ssr::*;
use std::{borrow::Cow, fmt};
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use std::{
  cell::{LazyCell, RefCell},
  rc::Rc,
};
pub use transparent::*;
pub use wasm_bindgen;
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use wasm_bindgen::JsCast;
use wasm_bindgen::UnwrapThrowExt;
pub use web_sys;

#[cfg(all(target_arch = "wasm32", feature = "web"))]
#[thread_local]
static COMMENT: LazyCell<web_sys::Node> =
  LazyCell::new(|| document().create_comment("").unchecked_into());
#[cfg(all(target_arch = "wasm32", feature = "web"))]
#[thread_local]
static RANGE: LazyCell<web_sys::Range> =
  LazyCell::new(|| web_sys::Range::new().unwrap());

/// Converts the value into a [`View`].
pub trait IntoView {
  /// Converts the value into [`View`].
  fn into_view(self, cx: Scope) -> View;
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
trait Mountable {
  /// Gets the [`web_sys::Node`] that can be directly inserted as
  /// a child of another node. Typically, this is a [`web_sys::DocumentFragment`]
  /// for components, and [`web_sys::HtmlElement`] for elements.
  ///
  /// ### Important Note
  /// Calling this method assumes that you are intending to move this
  /// view, and will unmount it's nodes from the DOM if this view is a
  /// component. In other words, don't call this method unless you intend
  /// to mount this view to another view or element.
  fn get_mountable_node(&self) -> web_sys::Node;

  /// Get's the first node of the [`View`].
  /// Typically, for [`HtmlElement`], this will be the
  /// `element` node. For components, this would be the
  /// first child node, or the `closing` marker comment node if
  /// no children are available.
  fn get_opening_node(&self) -> web_sys::Node;

  /// Get's the closing marker node.
  fn get_closing_node(&self) -> web_sys::Node;
}

impl IntoView for () {
  #[cfg_attr(
    debug_assertions,
    instrument(level = "trace", name = "<() />", skip_all)
  )]
  fn into_view(self, cx: Scope) -> View {
    Unit.into_view(cx)
  }
}

impl<T> IntoView for Option<T>
where
  T: IntoView,
{
  #[cfg_attr(
    debug_assertions,
    instrument(level = "trace", name = "Option<T>", skip_all)
  )]
  fn into_view(self, cx: Scope) -> View {
    if let Some(t) = self {
      t.into_view(cx)
    } else {
      Unit.into_view(cx)
    }
  }
}

impl<F, N> IntoView for F
where
  F: Fn() -> N + 'static,
  N: IntoView,
{
  #[cfg_attr(
    debug_assertions,
    instrument(level = "trace", name = "Fn() -> impl IntoView", skip_all)
  )]
  fn into_view(self, cx: Scope) -> View {
    DynChild::new(self).into_view(cx)
  }
}

impl<T> IntoView for (Scope, T)
where
  T: IntoView,
{
  fn into_view(self, _: Scope) -> View {
    self.1.into_view(self.0)
  }
}

cfg_if! {
  if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
    /// HTML element.
    #[derive(Clone, PartialEq, Eq)]
    pub struct Element {
      #[cfg(debug_assertions)]
      name: Cow<'static, str>,
      element: web_sys::HtmlElement,
    }

    impl fmt::Debug for Element {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let html = self.element.outer_html();

        f.write_str(&html)
      }
    }
  } else {
    /// HTML element.
    #[derive(Clone, PartialEq, Eq)]
    pub struct Element {
      name: Cow<'static, str>,
      is_void: bool,
      attrs: SmallVec<[(Cow<'static, str>, Cow<'static, str>); 4]>,
      children: Vec<View>,
      prerendered: Option<Cow<'static, str>>,
      id: HydrationKey,
    }

    impl fmt::Debug for Element {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use fmt::Write;

        let attrs =
          self.attrs.iter().map(|(n, v)| format!(" {n}=\"{v}\"")).collect::<String>();

        if self.is_void {
          write!(f, "<{}{attrs} />", self.name)
        } else {
          writeln!(f, "<{}{attrs}>", self.name)?;

          let mut pad_adapter = pad_adapter::PadAdapter::new(f);

          for child in &self.children {
            writeln!(pad_adapter, "{child:#?}")?;
          }

          write!(f, "</{}>", self.name)
        }

      }
    }
  }
}

impl Element {
  /// Converts this leptos [`Element`] into [`HtmlElement<AnyElement`].
  pub fn into_html_element(self, cx: Scope) -> HtmlElement<AnyElement> {
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    {
      let Self { element, .. } = self;

      let name = element.node_name().to_ascii_lowercase();

      let element = AnyElement {
        name: name.into(),
        element,
        is_void: false,
      };

      HtmlElement { cx, element }
    }

    #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
    {
      let Self {
        name,
        is_void,
        attrs,
        children,
        id,
        prerendered,
      } = self;

      let element = AnyElement { name, is_void, id };

      HtmlElement {
        cx,
        element,
        attrs,
        children: children.into_iter().collect(),
        prerendered,
      }
    }
  }
}

impl IntoView for Element {
  #[cfg_attr(debug_assertions, instrument(level = "trace", name = "<Element />", skip_all, fields(tag = %self.name)))]
  fn into_view(self, _: Scope) -> View {
    View::Element(self)
  }
}

impl Element {
  #[track_caller]
  fn new<El: ElementDescriptor>(el: El) -> Self {
    cfg_if! {
      if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
          Self {
            #[cfg(debug_assertions)]
            name: el.name(),
            element: el.as_ref().clone(),
          }
      }
      else {
        Self {
          name: el.name(),
          is_void: el.is_void(),
          attrs: Default::default(),
          children: Default::default(),
          id: el.hydration_id().clone(),
          prerendered: None
        }
      }
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Comment {
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  node: web_sys::Node,
  content: Cow<'static, str>,
}

impl Comment {
  fn new(
    content: impl Into<Cow<'static, str>>,
    id: &HydrationKey,
    closing: bool,
  ) -> Self {
    let content = content.into();

    #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
    {
      let _ = id;
      let _ = closing;
    }

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    let node = COMMENT.clone_node().unwrap();

    #[cfg(all(debug_assertions, target_arch = "wasm32", feature = "web"))]
    node.set_text_content(Some(&format!(" {content} ")));

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    {
      if HydrationCtx::is_hydrating() {
        let id = HydrationCtx::to_string(id, closing);

        if let Some(marker) = document().get_element_by_id(&id) {
          marker.before_with_node_1(&node).unwrap();

          marker.remove();
        } else {
          gloo::console::warn!(
            "component with id",
            id,
            "not found, ignoring it for hydration"
          );
        }
      }
    }

    Self {
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      node,
      content,
    }
  }
}

/// HTML text
#[derive(Clone, PartialEq, Eq)]
pub struct Text {
  /// In order to support partial updates on text nodes, that is,
  /// to update the node without recreating it, we need to be able
  /// to possibly reuse a previous node.
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  node: web_sys::Node,
  content: Cow<'static, str>,
}

impl fmt::Debug for Text {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "\"{}\"", self.content)
  }
}

impl IntoView for Text {
  #[cfg_attr(debug_assertions, instrument(level = "trace", name = "#text", skip_all, fields(content = %self.content)))]
  fn into_view(self, _: Scope) -> View {
    View::Text(self)
  }
}

impl Text {
  /// Creates a new [`Text`].
  pub fn new(content: Cow<'static, str>) -> Self {
    Self {
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      node: crate::document()
        .create_text_node(&content)
        .unchecked_into::<web_sys::Node>(),
      content,
    }
  }
}

/// A leptos view which can be mounted to the DOM.
#[derive(Clone, PartialEq, Eq)]
pub enum View {
  /// HTML element node.
  Element(Element),
  /// HTML text node.
  Text(Text),
  /// Custom leptos component.
  Component(ComponentRepr),
  /// leptos core-component.
  CoreComponent(CoreComponent),
  /// Wraps arbitrary data that's not part of the view but is
  /// passed via the view tree.
  Transparent(Transparent),
}

impl fmt::Debug for View {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Element(el) => el.fmt(f),
      Self::Text(t) => t.fmt(f),
      Self::Component(c) => c.fmt(f),
      Self::CoreComponent(c) => c.fmt(f),
      Self::Transparent(arg0) => {
        f.debug_tuple("Transparent").field(arg0).finish()
      }
    }
  }
}

/// The default [`View`] is the [`Unit`] core-component.
impl Default for View {
  fn default() -> Self {
    Self::CoreComponent(Default::default())
  }
}

impl IntoView for View {
  #[cfg_attr(debug_assertions, instrument(level = "trace", name = "Node", skip_all, fields(kind = self.kind_name())))]
  fn into_view(self, _: Scope) -> View {
    self
  }
}

impl<const N: usize> IntoView for [View; N] {
  #[cfg_attr(
    debug_assertions,
    instrument(level = "trace", name = "[Node; N]", skip_all)
  )]
  fn into_view(self, cx: Scope) -> View {
    Fragment::new(self.into_iter().collect()).into_view(cx)
  }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
impl Mountable for View {
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
      Self::Transparent(_) => panic!("tried to mount a Transparent node."),
    }
  }

  fn get_opening_node(&self) -> web_sys::Node {
    match self {
      Self::Text(t) => t.node.clone(),
      Self::Element(el) => el.element.clone().unchecked_into(),
      Self::CoreComponent(c) => match c {
        CoreComponent::DynChild(dc) => dc.get_opening_node(),
        CoreComponent::Each(e) => e.get_opening_node(),
        CoreComponent::Unit(u) => u.get_opening_node(),
      },
      Self::Component(c) => c.get_opening_node(),
      Self::Transparent(_) => {
        panic!("tried to get opening node for a Transparent node.")
      }
    }
  }

  fn get_closing_node(&self) -> web_sys::Node {
    match self {
      Self::Text(t) => t.node.clone(),
      Self::Element(el) => el.element.clone().unchecked_into(),
      Self::CoreComponent(c) => match c {
        CoreComponent::DynChild(dc) => dc.get_closing_node(),
        CoreComponent::Each(e) => e.get_closing_node(),
        CoreComponent::Unit(u) => u.get_closing_node(),
      },
      Self::Component(c) => c.get_closing_node(),
      Self::Transparent(_) => {
        panic!("tried to get closing node for a Transparent node.")
      }
    }
  }
}

impl View {
  #[cfg(debug_assertions)]
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
      Self::Transparent(..) => "Transparent",
    }
  }

  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  fn get_text(&self) -> Option<&Text> {
    if let Self::Text(t) = self {
      Some(t)
    } else {
      None
    }
  }

  /// Returns [`Some`] [`Text`] if the view is of this type. [`None`]
  /// otherwise.
  pub fn as_text(&self) -> Option<&Text> {
    if let Self::Text(t) = self {
      Some(t)
    } else {
      None
    }
  }

  /// Returns [`Some`] [`Element`] if the view is of this type. [`None`]
  /// otherwise.
  pub fn as_element(&self) -> Option<&Element> {
    if let Self::Element(el) = self {
      Some(el)
    } else {
      None
    }
  }

  /// Returns [`Some`] [`Transparent`] if the view is of this type. [`None`]
  /// otherwise.
  pub fn as_transparent(&self) -> Option<&Transparent> {
    match &self {
      Self::Transparent(t) => Some(t),
      _ => None,
    }
  }

  /// Returns [`Ok(HtmlElement<AnyElement>)`] if this [`View`] is
  /// of type [`Element`]. [`Err(View)`] otherwise.
  pub fn into_html_element(
    self,
    cx: Scope,
  ) -> Result<HtmlElement<AnyElement>, Self> {
    if let Self::Element(el) = self {
      Ok(el.into_html_element(cx))
    } else {
      Err(self)
    }
  }

  /// Adds an event listener, analogous to [`HtmlElement::on`].
  ///
  /// This method will attach an event listener to **all** child
  /// [`HtmlElement`] children.
  pub fn on<E: ev::EventDescriptor + 'static>(
    self,
    event: E,
    event_handler: impl FnMut(E::EventType) + 'static,
  ) -> Self {
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    match &self {
      Self::Element(el) => {
        if event.bubbles() {
          add_event_listener(&el.element, event.name(), event_handler);
        } else {
          add_event_listener_undelegated(
            &el.element,
            &event.name(),
            event_handler,
          );
        }
      }
      Self::Component(c) => {
        let event_handler = Rc::new(RefCell::new(event_handler));

        c.children.iter().cloned().for_each(|c| {
          let event_handler = event_handler.clone();

          c.on(event.clone(), move |e| event_handler.borrow_mut()(e));
        });
      }
      Self::CoreComponent(c) => match c {
        CoreComponent::DynChild(_) => {}
        CoreComponent::Each(_) => {}
        _ => {}
      },
      _ => {}
    }

    self
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
  }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
#[track_caller]
fn unmount_child(start: &web_sys::Node, end: &web_sys::Node) {
  let mut sibling = start.clone();

  while sibling != *end {
    if let Some(next_sibling) = sibling.next_sibling() {
      sibling.unchecked_ref::<web_sys::Element>().remove();

      sibling = next_sibling;
    } else {
      break;
    }
  }
}

/// Similar to [`unmount_child`], but instead of removing entirely
/// from the DOM, it inserts all child nodes into the [`DocumentFragment`].
///
/// [DocumentFragment]: web_sys::DocumentFragment
#[cfg(all(target_arch = "wasm32", feature = "web"))]
#[track_caller]
fn prepare_to_move(
  frag: &web_sys::DocumentFragment,
  opening: &web_sys::Node,
  closing: &web_sys::Node,
) {
  let mut sibling = opening.clone();

  while sibling != *closing {
    if let Some(next_sibling) = sibling.next_sibling() {
      frag.append_child(&sibling).unwrap();

      sibling = next_sibling;
    } else {
      frag.append_child(&sibling).unwrap();

      break;
    }
  }

  frag.append_child(closing).unwrap();
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
#[derive(Debug)]
enum MountKind<'a> {
  Before(
    // The closing node
    &'a web_sys::Node,
  ),
  Append(&'a web_sys::Node),
}

/// Runs the provided closure and mounts the result to eht `<body>`.
pub fn mount_to_body<F, N>(f: F)
where
  F: FnOnce(Scope) -> N + 'static,
  N: IntoView,
{
  cfg_if! {
    if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
      mount_to(crate::document().body().expect("body element to exist"), f)
    } else {
      _ = f;
      crate::warn!("`mount_to_body` should not be called outside the browser.");
    }
  }
}

/// Runs the provided closure and mounts the result to the provided element.
pub fn mount_to<F, N>(parent: web_sys::HtmlElement, f: F)
where
  F: FnOnce(Scope) -> N + 'static,
  N: IntoView,
{
  cfg_if! {
    if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
      let disposer = leptos_reactive::create_scope(
        leptos_reactive::create_runtime(),
        move |cx| {
          let node = f(cx).into_view(cx);

          HydrationCtx::stop_hydrating();

          parent.append_child(&node.get_mountable_node()).unwrap();

          std::mem::forget(node);
        },
      );

      std::mem::forget(disposer);
    } else {
      _ = parent;
      _ = f;
      crate::warn!("`mount_to` should not be called outside the browser.");
    }
  }
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

/// Returns true if running on the server (SSR).
///
/// In the past, this was implemented by checking whether `not(target_arch = "wasm32")`.
/// Now that some cloud platforms are moving to run Wasm on the edge, we really can't
/// guarantee that compiling to Wasm means browser APIs are available, or that not compiling
/// to Wasm means we're running on the server.
///
/// ```
/// # use leptos_dom::is_server;
/// let todos = if is_server() {
///   // if on the server, load from DB
/// } else {
///   // if on the browser, do something else
/// };
/// ```
pub const fn is_server() -> bool {
  !is_browser()
}

/// Returns true if running on the browser (CSR).
///
/// ```
/// # use leptos_dom::is_browser;
/// let todos = if is_browser() {
///   // if on the browser, call `wasm_bindgen` methods
/// } else {
///   // if on the server, do something else
/// };
/// ```
pub const fn is_browser() -> bool {
  cfg!(all(target_arch = "wasm32", feature = "web"))
}

/// Returns true if `debug_assertions` are enabled.
/// ```
/// # use leptos_dom::is_dev;
/// if is_dev!() {
///   // log something or whatever
/// }
/// ```
pub const fn is_dev() -> bool {
  cfg!(debug_assertions)
}

/// Returns true if `debug_assertions` are disabled.
pub const fn is_release() -> bool {
  !is_dev()
}

macro_rules! impl_into_view_for_tuples {
  ($($ty:ident),* $(,)?) => {
    impl<$($ty),*> IntoView for ($($ty,)*)
    where
      $($ty: IntoView),*
    {
      fn into_view(self, cx: Scope) -> View {
        paste::paste! {
          let ($([<$ty:lower>],)*) = self;
          [
            $([<$ty:lower>].into_view(cx)),*
          ].into_view(cx)
        }
      }
    }
  };
}

impl_into_view_for_tuples!(A);
impl_into_view_for_tuples!(A, B);
impl_into_view_for_tuples!(A, B, C);
impl_into_view_for_tuples!(A, B, C, D);
impl_into_view_for_tuples!(A, B, C, D, E);
impl_into_view_for_tuples!(A, B, C, D, E, F);
impl_into_view_for_tuples!(A, B, C, D, E, F, G);
impl_into_view_for_tuples!(A, B, C, D, E, F, G, H);
impl_into_view_for_tuples!(A, B, C, D, E, F, G, H, I);
impl_into_view_for_tuples!(A, B, C, D, E, F, G, H, I, J);
impl_into_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K);
impl_into_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_into_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_into_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_into_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_into_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
impl_into_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q);
impl_into_view_for_tuples!(
  A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R
);
impl_into_view_for_tuples!(
  A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S
);
impl_into_view_for_tuples!(
  A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T
);
impl_into_view_for_tuples!(
  A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U
);
impl_into_view_for_tuples!(
  A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V
);
impl_into_view_for_tuples!(
  A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W
);
impl_into_view_for_tuples!(
  A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X
);
impl_into_view_for_tuples!(
  A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y
);
impl_into_view_for_tuples!(
  A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z
);

macro_rules! api_planning {
  ($($tt:tt)*) => {};
}

api_planning! {
  let c = Component::<Props, ChildKind>::new("MyComponent")
    .props(Props::default()) // Can only be called once
    .child(Child1) // Anything that impl Into<ChildKind>
    .child(Child2);
}

impl IntoView for String {
  #[cfg_attr(
    debug_assertions,
    instrument(level = "trace", name = "#text", skip_all)
  )]
  fn into_view(self, _: Scope) -> View {
    View::Text(Text::new(self.into()))
  }
}

impl IntoView for &'static str {
  fn into_view(self, _: Scope) -> View {
    View::Text(Text::new(self.into()))
  }
}

impl<V> IntoView for Vec<V>
where
  V: IntoView,
{
  fn into_view(self, cx: Scope) -> View {
    self
      .into_iter()
      .map(|v| v.into_view(cx))
      .collect::<Fragment>()
      .into_view(cx)
  }
}

macro_rules! viewable_primitive {
  ($($child_type:ty),* $(,)?) => {
    $(
      impl IntoView for $child_type {
        fn into_view(self, _cx: Scope) -> View {
          View::Text(Text::new(self.to_string().into()))
        }
      }
    )*
  };
}

viewable_primitive![
  &String,
  usize,
  u8,
  u16,
  u32,
  u64,
  u128,
  isize,
  i8,
  i16,
  i32,
  i64,
  i128,
  f32,
  f64,
  char,
  bool,
  Cow<'_, str>,
  std::net::IpAddr,
  std::net::SocketAddr,
  std::net::SocketAddrV4,
  std::net::SocketAddrV6,
  std::net::Ipv4Addr,
  std::net::Ipv6Addr,
  std::backtrace::Backtrace,
  std::char::ToUppercase,
  std::char::ToLowercase,
  std::num::NonZeroI8,
  std::num::NonZeroU8,
  std::num::NonZeroI16,
  std::num::NonZeroU16,
  std::num::NonZeroI32,
  std::num::NonZeroU32,
  std::num::NonZeroI64,
  std::num::NonZeroU64,
  std::num::NonZeroI128,
  std::num::NonZeroU128,
  std::num::NonZeroIsize,
  std::num::NonZeroUsize,
  std::panic::Location<'_>,
  std::fmt::Arguments<'_>,
];
