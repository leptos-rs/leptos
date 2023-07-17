#![deny(missing_docs)]
#![forbid(unsafe_code)]
#![cfg_attr(feature = "nightly", feature(fn_traits))]
#![cfg_attr(feature = "nightly", feature(unboxed_closures))]

//! The DOM implementation for `leptos`.

#[doc(hidden)]
#[cfg_attr(any(debug_assertions, feature = "ssr"), macro_use)]
pub extern crate tracing;

mod components;
mod events;
pub mod helpers;
pub mod html;
mod hydration;
mod logging;
mod macro_helpers;
pub mod math;
mod node_ref;
/// Utilities for exporting nonces to be used for a Content Security Policy.
pub mod nonce;
pub mod ssr;
pub mod ssr_in_order;
pub mod svg;
mod transparent;
use cfg_if::cfg_if;
pub use components::*;
#[cfg(all(target_arch = "wasm32", feature = "web"))]
pub use events::add_event_helper;
pub use events::typed as ev;
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use events::{add_event_listener, add_event_listener_undelegated};
pub use html::HtmlElement;
use html::{AnyElement, ElementDescriptor};
pub use hydration::{HydrationCtx, HydrationKey};
use leptos_reactive::Scope;
#[cfg(not(feature = "nightly"))]
use leptos_reactive::{
    MaybeSignal, Memo, ReadSignal, RwSignal, Signal, SignalGet,
};
pub use logging::*;
pub use macro_helpers::*;
pub use node_ref::*;
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use once_cell::unsync::Lazy as LazyCell;
#[cfg(not(all(target_arch = "wasm32", feature = "web")))]
use smallvec::SmallVec;
use std::{borrow::Cow, fmt};
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use std::{cell::RefCell, rc::Rc};
pub use transparent::*;
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use wasm_bindgen::JsCast;
use wasm_bindgen::UnwrapThrowExt;

#[cfg(all(target_arch = "wasm32", feature = "web"))]
thread_local! {
  static COMMENT: LazyCell<web_sys::Node> =
    LazyCell::new(|| document().create_comment("").unchecked_into());
  static RANGE: LazyCell<web_sys::Range> =
    LazyCell::new(|| web_sys::Range::new().unwrap());
}

/// Converts the value into a [`View`].
pub trait IntoView {
    /// Converts the value into [`View`].
    fn into_view(self, cx: Scope) -> View;
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
#[doc(hidden)]
pub trait Mountable {
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
        any(debug_assertions, feature = "ssr"),
        instrument(level = "info", name = "<() />", skip_all)
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
        any(debug_assertions, feature = "ssr"),
        instrument(level = "info", name = "Option<T>", skip_all)
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
        any(debug_assertions, feature = "ssr"),
        instrument(level = "info", name = "Fn() -> impl IntoView", skip_all)
    )]
    #[track_caller]
    fn into_view(self, cx: Scope) -> View {
        DynChild::new(self).into_view(cx)
    }
}

impl<T> IntoView for (Scope, T)
where
    T: IntoView,
{
    #[inline(always)]
    fn into_view(self, _: Scope) -> View {
        self.1.into_view(self.0)
    }
}

#[cfg(not(feature = "nightly"))]
impl<T> IntoView for ReadSignal<T>
where
    T: IntoView + Clone,
{
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", name = "ReadSignal<T>", skip_all)
    )]
    fn into_view(self, cx: Scope) -> View {
        DynChild::new(move || self.get()).into_view(cx)
    }
}
#[cfg(not(feature = "nightly"))]
impl<T> IntoView for RwSignal<T>
where
    T: IntoView + Clone,
{
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", name = "RwSignal<T>", skip_all)
    )]
    fn into_view(self, cx: Scope) -> View {
        DynChild::new(move || self.get()).into_view(cx)
    }
}
#[cfg(not(feature = "nightly"))]
impl<T> IntoView for Memo<T>
where
    T: IntoView + Clone,
{
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", name = "Memo<T>", skip_all)
    )]
    fn into_view(self, cx: Scope) -> View {
        DynChild::new(move || self.get()).into_view(cx)
    }
}
#[cfg(not(feature = "nightly"))]
impl<T> IntoView for Signal<T>
where
    T: IntoView + Clone,
{
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", name = "Signal<T>", skip_all)
    )]
    fn into_view(self, cx: Scope) -> View {
        DynChild::new(move || self.get()).into_view(cx)
    }
}
#[cfg(not(feature = "nightly"))]
impl<T> IntoView for MaybeSignal<T>
where
    T: IntoView + Clone,
{
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", name = "MaybeSignal<T>", skip_all)
    )]
    fn into_view(self, cx: Scope) -> View {
        DynChild::new(move || self.get()).into_view(cx)
    }
}

/// Collects an iterator or collection into a [`View`].
pub trait CollectView {
    /// Collects an iterator or collection into a [`View`].
    fn collect_view(self, cx: Scope) -> View;
}

impl<I: IntoIterator<Item = T>, T: IntoView> CollectView for I {
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "info", name = "#text", skip_all)
    )]
    fn collect_view(self, cx: Scope) -> View {
        self.into_iter()
            .map(|v| v.into_view(cx))
            .collect::<Fragment>()
            .into_view(cx)
    }
}

cfg_if! {
  if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
    /// HTML element.
    #[derive(Clone, PartialEq, Eq)]
    pub struct Element {
      #[doc(hidden)]
      #[cfg(debug_assertions)]
      pub name: Cow<'static, str>,
      #[doc(hidden)]
      pub element: web_sys::HtmlElement,
      #[cfg(debug_assertions)]
      /// Optional marker for the view macro source of the element.
      pub view_marker: Option<String>
    }

    impl fmt::Debug for Element {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let html = self.element.outer_html();

        f.write_str(&html)
      }
    }
  } else {
    use crate::html::ElementChildren;

    /// HTML element.
    #[derive(Clone, PartialEq, Eq)]
    pub struct Element {
      name: Cow<'static, str>,
      is_void: bool,
      attrs: SmallVec<[(Cow<'static, str>, Cow<'static, str>); 4]>,
      children: ElementChildren,
      id: HydrationKey,
      #[cfg(debug_assertions)]
      /// Optional marker for the view macro source, in debug mode.
      pub view_marker: Option<String>
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

          if let ElementChildren::Children(children) = &self.children {
            for child in children {
                writeln!(pad_adapter, "{child:#?}")?;
            }
          }

          write!(f, "</{}>", self.name)
        }

      }
    }
  }
}

impl Element {
    /// Converts this leptos [`Element`] into [`HtmlElement<AnyElement>`].
    pub fn into_html_element(self, cx: Scope) -> HtmlElement<AnyElement> {
        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        {
            let Self {
                element,
                #[cfg(debug_assertions)]
                view_marker,
                ..
            } = self;

            let name = element.node_name().to_ascii_lowercase();

            let element = AnyElement {
                name: name.into(),
                element,
                is_void: false,
            };

            HtmlElement {
                cx,
                element,
                #[cfg(debug_assertions)]
                span: ::tracing::Span::current(),
                #[cfg(debug_assertions)]
                view_marker,
            }
        }

        #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
        {
            let Self {
                name,
                is_void,
                attrs,
                children,
                id,
                #[cfg(debug_assertions)]
                view_marker,
            } = self;

            let element = AnyElement { name, is_void, id };

            HtmlElement {
                cx,
                element,
                attrs,
                children,
                #[cfg(debug_assertions)]
                view_marker,
            }
        }
    }
}

impl IntoView for Element {
    #[cfg_attr(debug_assertions, instrument(level = "info", name = "<Element />", skip_all, fields(tag = %self.name)))]
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
                #[cfg(debug_assertions)]
                view_marker: None
              }
          }
          else {
            Self {
              name: el.name(),
              is_void: el.is_void(),
              attrs: Default::default(),
              children: Default::default(),
              id: *el.hydration_id(),
              #[cfg(debug_assertions)]
              view_marker: None
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
    #[inline]
    fn new(
        content: impl Into<Cow<'static, str>>,
        id: &HydrationKey,
        closing: bool,
    ) -> Self {
        Self::new_inner(content.into(), id, closing)
    }

    fn new_inner(
        content: Cow<'static, str>,
        id: &HydrationKey,
        closing: bool,
    ) -> Self {
        cfg_if! {
            if #[cfg(not(all(target_arch = "wasm32", feature = "web")))] {
                let _ = id;
                let _ = closing;

                Self { content }
            } else {
                #[cfg(not(feature = "hydrate"))]
                {
                    _ = id;
                    _ = closing;
                }

                let node = COMMENT.with(|comment| comment.clone_node().unwrap());

                #[cfg(debug_assertions)]
                node.set_text_content(Some(&format!(" {content} ")));

                #[cfg(feature = "hydrate")]
                if HydrationCtx::is_hydrating() {
                    let id = HydrationCtx::to_string(id, closing);

                    if let Some(marker) = hydration::get_marker(&id) {
                        marker.before_with_node_1(&node).unwrap();

                        marker.remove();
                    } else {
                        crate::warn!(
                            "component with id {id} not found, ignoring it for \
                             hydration"
                        );
                    }
                }

                Self {
                    node,
                    content,
                }
            }
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
    /// The current contents of the text node.
    pub content: Cow<'static, str>,
}

impl fmt::Debug for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\"", self.content)
    }
}

impl IntoView for Text {
    #[cfg_attr(debug_assertions, instrument(level = "info", name = "#text", skip_all, fields(content = %self.content)))]
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
#[must_use = "You are creating a View but not using it. An unused view can \
              cause your view to be rendered as () unexpectedly, and it can \
              also cause issues with client-side hydration."]
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
    /// Marks the contents of Suspense component, which can be replaced in streaming SSR.
    Suspense(HydrationKey, CoreComponent),
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
            Self::Suspense(id, c) => {
                f.debug_tuple("Suspense").field(id).field(c).finish()
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
    #[cfg_attr(debug_assertions, instrument(level = "info", name = "Node", skip_all, fields(kind = self.kind_name())))]
    fn into_view(self, _: Scope) -> View {
        self
    }
}

impl IntoView for &View {
    fn into_view(self, _: Scope) -> View {
        self.clone()
    }
}

impl<const N: usize> IntoView for [View; N] {
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "info", name = "[Node; N]", skip_all)
    )]
    fn into_view(self, cx: Scope) -> View {
        Fragment::new(self.into_iter().collect()).into_view(cx)
    }
}

impl IntoView for &Fragment {
    fn into_view(self, cx: Scope) -> View {
        self.to_owned().into_view(cx)
    }
}

impl FromIterator<View> for View {
    fn from_iter<T: IntoIterator<Item = View>>(iter: T) -> Self {
        iter.into_iter().collect::<Fragment>().into()
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
            Self::CoreComponent(c) | Self::Suspense(_, c) => match c {
                CoreComponent::Unit(u) => u.get_mountable_node(),
                CoreComponent::DynChild(dc) => dc.get_mountable_node(),
                CoreComponent::Each(e) => e.get_mountable_node(),
            },
            Self::Component(c) => c.get_mountable_node(),
            Self::Transparent(_) => {
                panic!("tried to mount a Transparent node.")
            }
        }
    }

    fn get_opening_node(&self) -> web_sys::Node {
        match self {
            Self::Text(t) => t.node.clone(),
            Self::Element(el) => el.element.clone().unchecked_into(),
            Self::CoreComponent(c) | Self::Suspense(_, c) => match c {
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
            Self::CoreComponent(c) | Self::Suspense(_, c) => match c {
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
            Self::Suspense(..) => "Suspense",
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
    #[inline(always)]
    pub fn on<E: ev::EventDescriptor + 'static>(
        self,
        event: E,
        #[allow(unused_mut)] mut event_handler: impl FnMut(E::EventType) + 'static,
    ) -> Self {
        cfg_if::cfg_if! {
          if #[cfg(debug_assertions)] {
            trace!("calling on() {}", event.name());
            let span = ::tracing::Span::current();
            let event_handler = move |e| {
              let _guard = span.enter();
              event_handler(e);
            };
          }
        }

        self.on_impl(event, Box::new(event_handler))
    }

    fn on_impl<E: ev::EventDescriptor + 'static>(
        self,
        event: E,
        event_handler: Box<dyn FnMut(E::EventType)>,
    ) -> Self {
        cfg_if! {
          if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
            match &self {
              Self::Element(el) => {
                if E::BUBBLES {
                  add_event_listener(&el.element, event.event_delegation_key(), event.name(), event_handler, &None);
                } else {
                  add_event_listener_undelegated(
                    &el.element,
                    &event.name(),
                    event_handler,
                    &None,
                  );
                }
              }
              Self::Component(c) => {
                let event_handler = Rc::new(RefCell::new(event_handler));

                c.children.iter().cloned().for_each(|c| {
                  let event_handler = event_handler.clone();

                  _ = c.on(event.clone(), Box::new(move |e| event_handler.borrow_mut()(e)));
                });
              }
              Self::CoreComponent(c) => match c {
                CoreComponent::DynChild(_) => {}
                CoreComponent::Each(_) => {}
                _ => {}
              },
              _ => {}
            }
          } else {
            _ = event;
            _ = event_handler;
          }
        }

        self
    }
}

#[cfg_attr(debug_assertions, instrument)]
#[track_caller]
#[cfg(all(target_arch = "wasm32", feature = "web"))]
#[doc(hidden)]
pub fn mount_child<GWSN: Mountable + fmt::Debug>(
    kind: MountKind,
    child: &GWSN,
) {
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
#[doc(hidden)]
pub enum MountKind<'a> {
    Before(
        // The closing node
        &'a web_sys::Node,
    ),
    Append(&'a web_sys::Node),
}

/// Runs the provided closure and mounts the result to the `<body>`.
pub fn mount_to_body<F, N>(f: F)
where
    F: FnOnce(Scope) -> N + 'static,
    N: IntoView,
{
    #[cfg(all(feature = "web", feature = "ssr"))]
    crate::console_warn(
        "You have both `csr` and `ssr` or `hydrate` and `ssr` enabled as \
         features, which may cause issues like <Suspense/>` failing to work \
         silently.",
    );

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
/// This is cached as a thread-local variable, so calling `document()` multiple times
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
///     // if on the server, load from DB
/// } else {
///     // if on the browser, do something else
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
///     // if on the browser, call `wasm_bindgen` methods
/// } else {
///     // if on the server, do something else
/// };
/// ```
pub const fn is_browser() -> bool {
    cfg!(all(target_arch = "wasm32", feature = "web"))
}

/// Returns true if `debug_assertions` are enabled.
/// ```
/// # use leptos_dom::is_dev;
/// if is_dev() {
///     // log something or whatever
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
      #[inline]
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
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y,
    Z
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
        any(debug_assertions, feature = "ssr"),
        instrument(level = "info", name = "#text", skip_all)
    )]
    #[inline(always)]
    fn into_view(self, _: Scope) -> View {
        View::Text(Text::new(self.into()))
    }
}

impl IntoView for &'static str {
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "info", name = "#text", skip_all)
    )]
    #[inline(always)]
    fn into_view(self, _: Scope) -> View {
        View::Text(Text::new(self.into()))
    }
}

impl<V> IntoView for Vec<V>
where
    V: IntoView,
{
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "info", name = "#text", skip_all)
    )]
    fn into_view(self, cx: Scope) -> View {
        self.into_iter()
            .map(|v| v.into_view(cx))
            .collect::<Fragment>()
            .into_view(cx)
    }
}

macro_rules! viewable_primitive {
  ($($child_type:ty),* $(,)?) => {
    $(
      impl IntoView for $child_type {
        #[inline(always)]
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

cfg_if! {
  if #[cfg(feature = "nightly")] {
    viewable_primitive! {
        std::backtrace::Backtrace
    }
  }
}
