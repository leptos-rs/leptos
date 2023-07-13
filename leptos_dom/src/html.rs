//! Exports types for working with HTML elements.

use cfg_if::cfg_if;

cfg_if! {
  if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
    use crate::events::*;
    use crate::macro_helpers::*;
    use crate::{mount_child, MountKind};
    use once_cell::unsync::Lazy as LazyCell;
    use std::cell::Cell;
    use wasm_bindgen::JsCast;

    /// Trait alias for the trait bounts on [`ElementDescriptor`].
    pub trait ElementDescriptorBounds:
      fmt::Debug + AsRef<web_sys::HtmlElement> + Clone
    {
    }

    impl<El> ElementDescriptorBounds for El where
      El: fmt::Debug + AsRef<web_sys::HtmlElement> + Clone
    {
    }

    thread_local! {
        static IS_META: Cell<bool> = Cell::new(false);
    }

    #[doc(hidden)]
    pub fn as_meta_tag<T>(f: impl FnOnce() -> T) -> T {
        IS_META.with(|m| m.set(true));
        let v = f();
        IS_META.with(|m| m.set(false));
        v
    }

    fn is_meta_tag() -> bool {
        IS_META.with(|m| m.get())
    }
  } else {
    use crate::hydration::HydrationKey;
    use smallvec::{smallvec, SmallVec};

    pub(crate) const HTML_ELEMENT_DEREF_UNIMPLEMENTED_MSG: &str =
      "`Deref<Target = web_sys::HtmlElement>` and `AsRef<web_sys::HtmlElement>` \
      can only be used on web targets. \
      This is for the same reason that normal `wasm_bindgen` methods can be used \
      only in the browser. Please use `leptos::is_server()` or \
      `leptos::is_browser()` to check where you're running.";

    /// Trait alias for the trait bounts on [`ElementDescriptor`].
    pub trait ElementDescriptorBounds: fmt::Debug {}

    impl<El> ElementDescriptorBounds for El where El: fmt::Debug {}

    #[doc(hidden)]
    pub fn as_meta_tag<T>(f: impl FnOnce() -> T) -> T {
        f()
    }
  }
}

use crate::{
    ev::EventDescriptor,
    hydration::HydrationCtx,
    macro_helpers::{IntoAttribute, IntoClass, IntoProperty, IntoStyle},
    Element, Fragment, IntoView, NodeRef, Text, View,
};
use leptos_reactive::Scope;
use std::{borrow::Cow, fmt};

/// Trait which allows creating an element tag.
pub trait ElementDescriptor: ElementDescriptorBounds {
    /// The name of the element, i.e., `div`, `p`, `custom-element`.
    fn name(&self) -> Cow<'static, str>;

    /// Determines if the tag is void, i.e., `<input>` and `<br>`.
    #[inline(always)]
    fn is_void(&self) -> bool {
        false
    }

    /// A unique `id` that should be generated for each new instance of
    /// this element, and be consistent for both SSR and CSR.
    #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
    fn hydration_id(&self) -> &HydrationKey;
}

/// Trait for converting any type which impl [`AsRef<web_sys::Element>`]
/// to [`HtmlElement`].
pub trait ToHtmlElement {
    /// Converts the type to [`HtmlElement`].
    fn to_leptos_element(self, cx: Scope) -> HtmlElement<AnyElement>;
}

impl<T> ToHtmlElement for T
where
    T: AsRef<web_sys::Element>,
{
    fn to_leptos_element(self, cx: Scope) -> HtmlElement<AnyElement> {
        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        {
            let el = self.as_ref().clone().unchecked_into();

            let element = AnyElement {
                name: "".into(),
                is_void: false,
                element: el,
            };

            HtmlElement {
                cx,
                element,
                #[cfg(debug_assertions)]
                span: ::tracing::Span::current(),
                #[cfg(debug_assertions)]
                view_marker: None,
            }
        }

        #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
        {
            let _ = cx;

            unreachable!();
        }
    }
}

/// Represents potentially any element.
#[derive(Clone, Debug)]
pub struct AnyElement {
    pub(crate) name: Cow<'static, str>,
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    pub(crate) element: web_sys::HtmlElement,
    pub(crate) is_void: bool,
    #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
    pub(crate) id: HydrationKey,
}

impl std::ops::Deref for AnyElement {
    type Target = web_sys::HtmlElement;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        return &self.element;

        #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
        unimplemented!("{HTML_ELEMENT_DEREF_UNIMPLEMENTED_MSG}");
    }
}

impl std::convert::AsRef<web_sys::HtmlElement> for AnyElement {
    #[inline(always)]
    fn as_ref(&self) -> &web_sys::HtmlElement {
        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        return &self.element;

        #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
        unimplemented!("{HTML_ELEMENT_DEREF_UNIMPLEMENTED_MSG}");
    }
}

impl ElementDescriptor for AnyElement {
    fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    #[inline(always)]
    fn is_void(&self) -> bool {
        self.is_void
    }

    #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
    #[inline(always)]
    fn hydration_id(&self) -> &HydrationKey {
        &self.id
    }
}

/// Represents a custom HTML element, such as `<my-element>`.
#[derive(Clone, Debug)]
pub struct Custom {
    name: Cow<'static, str>,
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    element: web_sys::HtmlElement,
    #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
    id: HydrationKey,
}

impl Custom {
    /// Creates a new custom element with the given tag name.
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        let name = name.into();
        let id = HydrationCtx::id();

        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        let element = if HydrationCtx::is_hydrating() {
            if let Some(el) =
                crate::document().get_element_by_id(&format!("_{id}"))
            {
                #[cfg(debug_assertions)]
                assert_eq!(
                    el.node_name().to_ascii_uppercase(),
                    name.to_ascii_uppercase(),
                    "SSR and CSR elements have the same hydration key but \
                     different node kinds. Check out the docs for information \
                     about this kind of hydration bug: https://leptos-rs.github.io/leptos/ssr/24_hydration_bugs.html"
                );

                el.remove_attribute("id").unwrap();

                el.unchecked_into()
            } else if let Ok(Some(el)) =
                crate::document().query_selector(&format!("[leptos-hk=_{id}]"))
            {
                #[cfg(debug_assertions)]
                assert_eq!(
                    el.node_name().to_ascii_uppercase(),
                    name.to_ascii_uppercase(),
                    "SSR and CSR elements have the same hydration key but \
                     different node kinds. Check out the docs for information \
                     about this kind of hydration bug: https://leptos-rs.github.io/leptos/ssr/24_hydration_bugs.html"
                );

                el.remove_attribute("leptos-hk").unwrap();

                el.unchecked_into()
            } else {
                if !is_meta_tag() {
                    crate::warn!(
                        "element with id {id} not found, ignoring it for \
                         hydration"
                    );
                }

                crate::document().create_element(&name).unwrap()
            }
        } else {
            crate::document().create_element(&name).unwrap()
        };

        Self {
            name,
            #[cfg(all(target_arch = "wasm32", feature = "web"))]
            element: element.unchecked_into(),
            #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
            id,
        }
    }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
impl std::ops::Deref for Custom {
    type Target = web_sys::HtmlElement;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.element
    }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
impl std::convert::AsRef<web_sys::HtmlElement> for Custom {
    #[inline(always)]
    fn as_ref(&self) -> &web_sys::HtmlElement {
        &self.element
    }
}

impl ElementDescriptor for Custom {
    fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
    #[inline(always)]
    fn hydration_id(&self) -> &HydrationKey {
        &self.id
    }
}

cfg_if! {
  if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
    /// Represents an HTML element.
    #[derive(Clone)]
    pub struct HtmlElement<El: ElementDescriptor> {
      #[cfg(debug_assertions)]
      pub(crate) span: ::tracing::Span,
      pub(crate) cx: Scope,
      pub(crate) element: El,
      #[cfg(debug_assertions)]
      pub(crate) view_marker: Option<String>
    }
  // Server needs to build a virtualized DOM tree
  } else {
    /// Represents an HTML element.
    #[derive(educe::Educe, Clone)]
    #[educe(Debug)]
    pub struct HtmlElement<El: ElementDescriptor> {
      pub(crate) cx: Scope,
      pub(crate) element: El,
      pub(crate) attrs: SmallVec<[(Cow<'static, str>, Cow<'static, str>); 4]>,
      #[educe(Debug(ignore))]
      pub(crate) children: ElementChildren,
      #[cfg(debug_assertions)]
      pub(crate) view_marker: Option<String>
    }

    #[derive(Clone, educe::Educe, PartialEq, Eq)]
    #[educe(Default)]
    pub(crate) enum ElementChildren {
        #[educe(Default)]
        Empty,
        Children(Vec<View>),
        InnerHtml(Cow<'static, str>),
        Chunks(Vec<StringOrView>)
    }

    #[doc(hidden)]
    #[derive(Clone)]
    pub enum StringOrView {
        String(Cow<'static, str>),
        View(std::rc::Rc<dyn Fn() -> View>)
    }

    impl PartialEq for StringOrView {
        fn eq(&self, other: &Self) -> bool {
            match (self, other) {
                (StringOrView::String(a), StringOrView::String(b)) => a == b,
                _ => false
            }
        }
    }

    impl Eq for StringOrView {}
  }
}

impl<El> std::ops::Deref for HtmlElement<El>
where
    El: ElementDescriptor + std::ops::Deref,
{
    type Target = <El as std::ops::Deref>::Target;

    fn deref(&self) -> &Self::Target {
        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        return self.element.deref();

        #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
        unimplemented!("{HTML_ELEMENT_DEREF_UNIMPLEMENTED_MSG}");
    }
}

impl<El: ElementDescriptor + 'static> HtmlElement<El> {
    pub(crate) fn new(cx: Scope, element: El) -> Self {
        cfg_if! {
          if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
            Self {
              cx,
              element,
              #[cfg(debug_assertions)]
              span: ::tracing::Span::current(),
              #[cfg(debug_assertions)]
              view_marker: None
            }
          } else {
            Self {
              cx,
              attrs: smallvec![],
              children: Default::default(),
              element,
              #[cfg(debug_assertions)]
              view_marker: None
            }
          }
        }
    }

    #[doc(hidden)]
    #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
    pub fn from_chunks(
        cx: Scope,
        element: El,
        chunks: impl IntoIterator<Item = StringOrView>,
    ) -> Self {
        Self {
            cx,
            attrs: smallvec![],
            children: ElementChildren::Chunks(chunks.into_iter().collect()),
            element,
            #[cfg(debug_assertions)]
            view_marker: None,
        }
    }

    #[cfg(debug_assertions)]
    /// Adds an optional marker indicating the view macro source.
    #[inline(always)]
    pub fn with_view_marker(mut self, marker: impl Into<String>) -> Self {
        self.view_marker = Some(marker.into());
        self
    }

    /// Converts this element into [`HtmlElement<AnyElement>`].
    pub fn into_any(self) -> HtmlElement<AnyElement> {
        cfg_if! {
          if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
            let Self {
              cx,
              element,
              #[cfg(debug_assertions)]
              span,
              #[cfg(debug_assertions)]
              view_marker
            } = self;

            HtmlElement {
              cx,
              element: AnyElement {
                name: element.name(),
                element: element.as_ref().clone(),
                is_void: element.is_void(),
              },
              #[cfg(debug_assertions)]
              span,
              #[cfg(debug_assertions)]
              view_marker
            }
          } else {
            let Self {
              cx,
              attrs,
              children,
              element,
              #[cfg(debug_assertions)]
              view_marker
            } = self;

            HtmlElement {
              cx,
              attrs,
              children,
              element: AnyElement {
                name: element.name(),
                is_void: element.is_void(),
                id: *element.hydration_id()
              },
              #[cfg(debug_assertions)]
              view_marker
            }
          }
        }
    }

    /// Adds an `id` to the element.
    #[track_caller]
    #[inline(always)]
    pub fn id(self, id: impl Into<Cow<'static, str>>) -> Self {
        let id = id.into();

        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        {
            #[inline(never)]
            fn id_inner(el: &web_sys::HtmlElement, id: &str) {
                el.set_attribute(wasm_bindgen::intern("id"), id).unwrap()
            }

            id_inner(self.element.as_ref(), &id);

            self
        }

        #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
        {
            let mut this = self;

            this.attrs.push(("id".into(), id));

            this
        }
    }

    /// Binds the element reference to [`NodeRef`].
    #[inline(always)]
    pub fn node_ref(self, node_ref: NodeRef<El>) -> Self
    where
        Self: Clone,
    {
        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        node_ref.load(&self);

        #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
        let _ = node_ref;

        self
    }

    /// Runs the callback when this element has been mounted to the DOM.
    ///
    /// ### Important Note
    /// This method will only ever run at most once. If this element
    /// is unmounted and remounted, or moved somewhere else, it will not
    /// re-run unless you call this method again.
    pub fn on_mount(self, f: impl FnOnce(Self) + 'static) -> Self {
        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        {
            use futures::future::poll_fn;
            use once_cell::unsync::OnceCell;
            use std::{
                cell::RefCell,
                rc::Rc,
                task::{Poll, Waker},
            };

            let this = self.clone();
            let el = self.element.as_ref().clone();

            wasm_bindgen_futures::spawn_local(async move {
                while !crate::document().body().unwrap().contains(Some(&el)) {
                    // We need to cook ourselves a small future that resolves
                    // when the next animation frame is available
                    let waker = Rc::new(RefCell::new(None::<Waker>));
                    let ready = Rc::new(OnceCell::new());

                    crate::helpers::request_animation_frame({
                        let waker = waker.clone();
                        let ready = ready.clone();

                        move || {
                            let _ = ready.set(());
                            if let Some(waker) = &*waker.borrow() {
                                waker.wake_by_ref();
                            }
                        }
                    });

                    // Wait for the animation frame to become available
                    poll_fn(move |cx| {
                        let mut waker_borrow = waker.borrow_mut();

                        *waker_borrow = Some(cx.waker().clone());

                        if ready.get().is_some() {
                            Poll::Ready(())
                        } else {
                            Poll::<()>::Pending
                        }
                    })
                    .await;
                }

                f(this);
            });
        }

        #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
        {
            let _ = f;
        }
        self
    }

    /// Checks to see if this element is mounted to the DOM as a child
    /// of `body`.
    ///
    /// This method will always return [`None`] on non-wasm CSR targets.
    #[inline(always)]
    pub fn is_mounted(&self) -> bool {
        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        {
            #[inline(never)]
            fn is_mounted_inner(el: &web_sys::HtmlElement) -> bool {
                crate::document().body().unwrap().contains(Some(el))
            }

            is_mounted_inner(self.element.as_ref())
        }

        #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
        false
    }

    /// Adds an attribute to this element.
    #[track_caller]
    #[cfg_attr(all(target_arch = "wasm32", feature = "web"), inline(always))]
    pub fn attr(
        self,
        name: impl Into<Cow<'static, str>>,
        attr: impl IntoAttribute,
    ) -> Self {
        let name = name.into();

        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        {
            attribute_helper(
                self.element.as_ref(),
                name,
                attr.into_attribute(self.cx),
            );
            self
        }

        #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
        {
            use crate::macro_helpers::Attribute;

            let mut this = self;

            let mut attr = attr.into_attribute(this.cx);
            while let Attribute::Fn(_, f) = attr {
                attr = f();
            }
            match attr {
                Attribute::String(value) => {
                    this.attrs.push((name, value));
                }
                Attribute::Bool(include) => {
                    if include {
                        this.attrs.push((name, "".into()));
                    }
                }
                Attribute::Option(_, maybe) => {
                    if let Some(value) = maybe {
                        this.attrs.push((name, value));
                    }
                }
                _ => unreachable!(),
            }

            this
        }
    }

    /// Adds a class to an element.
    ///
    /// **Note**: In the builder syntax, this will be overwritten by the `class`
    /// attribute if you use `.attr("class", /* */)`. In the `view` macro, they
    /// are automatically re-ordered so that this over-writing does not happen.
    ///
    /// # Panics
    /// This directly uses the browser’s `classList` API, which means it will throw
    /// a runtime error if you pass more than a single class name. If you want to
    /// pass more than one class name at a time, you can use [HtmlElement::classes].
    #[track_caller]
    pub fn class(
        self,
        name: impl Into<Cow<'static, str>>,
        class: impl IntoClass,
    ) -> Self {
        let name = name.into();

        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        {
            let el = self.element.as_ref();
            let value = class.into_class(self.cx);
            class_helper(el, name, value);

            self
        }

        #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
        {
            use crate::macro_helpers::Class;

            let mut this = self;

            let class = class.into_class(this.cx);

            let include = match class {
                Class::Value(include) => include,
                Class::Fn(_, f) => f(),
            };

            if include {
                if let Some((_, ref mut value)) =
                    this.attrs.iter_mut().find(|(name, _)| name == "class")
                {
                    *value = format!("{value} {name}").into();
                } else {
                    this.attrs.push(("class".into(), name));
                }
            }

            this
        }
    }

    fn classes_inner(self, classes: &str) -> Self {
        let mut this = self;
        for class in classes.split_ascii_whitespace() {
            this = this.class(class.to_string(), true);
        }
        this
    }

    /// Adds a list of classes separated by ASCII whitespace to an element.
    #[track_caller]
    #[inline(always)]
    pub fn classes(self, classes: impl Into<Cow<'static, str>>) -> Self {
        self.classes_inner(&classes.into())
    }

    /// Sets the class on the element as the class signal changes.
    #[track_caller]
    pub fn dyn_classes<I, C>(
        self,
        classes_signal: impl Fn() -> I + 'static,
    ) -> Self
    where
        I: IntoIterator<Item = C>,
        C: Into<Cow<'static, str>>,
    {
        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        {
            use smallvec::SmallVec;

            let class_list = self.element.as_ref().class_list();

            leptos_reactive::create_effect(
                self.cx,
                move |prev_classes: Option<
                    SmallVec<[Cow<'static, str>; 4]>,
                >| {
                    let classes = classes_signal()
                        .into_iter()
                        .map(Into::into)
                        .collect::<SmallVec<[Cow<'static, str>; 4]>>(
                    );

                    let new_classes = classes
                        .iter()
                        .flat_map(|classes| classes.split_whitespace());

                    if let Some(prev_classes) = prev_classes {
                        let new_classes =
                            new_classes.collect::<SmallVec<[_; 4]>>();
                        let mut old_classes = prev_classes
                            .iter()
                            .flat_map(|classes| classes.split_whitespace());

                        // Remove old classes
                        for prev_class in old_classes.clone() {
                            if !new_classes.iter().any(|c| c == &prev_class) {
                                class_list.remove_1(prev_class).unwrap_or_else(
                                    |err| {
                                        panic!(
                                            "failed to remove class \
                                             `{prev_class}`, error: {err:#?}"
                                        )
                                    },
                                );
                            }
                        }

                        // Add new classes
                        for class in new_classes {
                            if !old_classes.any(|c| c == class) {
                                class_list.add_1(class).unwrap_or_else(|err| {
                                    panic!(
                                        "failed to add class `{class}`, \
                                         error: {err:#?}"
                                    )
                                });
                            }
                        }
                    } else {
                        let new_classes = new_classes
                            .map(ToOwned::to_owned)
                            .collect::<SmallVec<[_; 4]>>();

                        for class in &new_classes {
                            class_list.add_1(class).unwrap_or_else(|err| {
                                panic!(
                                    "failed to add class `{class}`, error: \
                                     {err:#?}"
                                )
                            });
                        }
                    }

                    classes
                },
            );

            self
        }

        #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
        {
            classes_signal()
                .into_iter()
                .map(Into::into)
                .flat_map(|classes| {
                    classes
                        .split_whitespace()
                        .map(ToString::to_string)
                        .collect::<SmallVec<[_; 4]>>()
                })
                .fold(self, |this, class| this.class(class, true))
        }
    }

    /// Sets a style on an element.
    ///
    /// **Note**: In the builder syntax, this will be overwritten by the `style`
    /// attribute if you use `.attr("style", /* */)`. In the `view` macro, they
    /// are automatically re-ordered so that this over-writing does not happen.
    #[track_caller]
    pub fn style(
        self,
        name: impl Into<Cow<'static, str>>,
        style: impl IntoStyle,
    ) -> Self {
        let name = name.into();

        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        {
            let el = self.element.as_ref();
            let value = style.into_style(self.cx);
            style_helper(el, name, value);

            self
        }

        #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
        {
            use crate::macro_helpers::Style;

            let mut this = self;

            let style = style.into_style(this.cx);

            let include = match style {
                Style::Value(value) => Some(value),
                Style::Option(value) => value,
                Style::Fn(_, f) => {
                    let mut value = f();
                    while let Style::Fn(_, f) = value {
                        value = f();
                    }
                    match value {
                        Style::Value(value) => Some(value),
                        Style::Option(value) => value,
                        _ => unreachable!(),
                    }
                }
            };

            if let Some(style_value) = include {
                if let Some((_, ref mut value)) =
                    this.attrs.iter_mut().find(|(name, _)| name == "style")
                {
                    *value = format!("{value} {name}: {style_value};").into();
                } else {
                    this.attrs.push((
                        "style".into(),
                        format!("{name}: {style_value};").into(),
                    ));
                }
            }

            this
        }
    }

    /// Sets a property on an element.
    #[track_caller]
    pub fn prop(
        self,
        name: impl Into<Cow<'static, str>>,
        value: impl IntoProperty,
    ) -> Self {
        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        {
            let name = name.into();
            let value = value.into_property(self.cx);
            let el = self.element.as_ref();
            property_helper(el, name, value);
        }

        #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
        {
            let _ = name;
            let _ = value;
        }

        self
    }

    /// Adds an event listener to this element.
    #[track_caller]
    #[inline(always)]
    pub fn on<E: EventDescriptor + 'static>(
        self,
        event: E,
        #[allow(unused_mut)] // used for tracing in debug
        mut event_handler: impl FnMut(E::EventType) + 'static,
    ) -> Self {
        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        {
            cfg_if! {
                if #[cfg(debug_assertions)] {
                    let onspan = ::tracing::span!(
                        parent: &self.span,
                        ::tracing::Level::TRACE,
                        "on",
                        event = %event.name()
                    );
                    let _onguard = onspan.enter();
                }
            }
            let event_name = event.name();

            let key = event.event_delegation_key();
            let event_handler = Box::new(event_handler);

            if E::BUBBLES {
                add_event_listener(
                    self.element.as_ref(),
                    key,
                    event_name,
                    event_handler,
                    event.options(),
                );
            } else {
                add_event_listener_undelegated(
                    self.element.as_ref(),
                    &event_name,
                    event_handler,
                    event.options(),
                );
            }

            self
        }

        #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
        {
            _ = event;
            _ = event_handler;

            self
        }
    }

    /// Optionally adds an event listener to this element.
    ///
    /// ## Example
    /// ```rust
    /// # use leptos::*;
    /// #[component]
    /// pub fn Input(
    ///     cx: Scope,
    ///     #[prop(optional)] value: Option<RwSignal<String>>,
    /// ) -> impl IntoView {
    ///     view! { cx, <input/> }
    ///         // only add event if `value` is `Some(signal)`
    ///         .optional_event(
    ///             ev::input,
    ///             value.map(|value| move |ev| value.set(event_target_value(&ev))),
    ///         )
    /// }
    /// #
    /// ```
    #[track_caller]
    #[inline(always)]
    pub fn optional_event<E: EventDescriptor + 'static>(
        self,
        event: E,
        #[allow(unused_mut)] // used for tracing in debug
        mut event_handler: Option<impl FnMut(E::EventType) + 'static>,
    ) -> Self {
        if let Some(event_handler) = event_handler {
            self.on(event, event_handler)
        } else {
            self
        }
    }

    /// Adds a child to this element.
    #[track_caller]
    pub fn child(self, child: impl IntoView) -> Self {
        let child = child.into_view(self.cx);

        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        {
            if !HydrationCtx::is_hydrating() {
                // add a debug-only, run-time warning for the SVG <a> element
                #[cfg(debug_assertions)]
                warn_on_ambiguous_a(self.element.as_ref(), &child);

                mount_child(MountKind::Append(self.element.as_ref()), &child);
            }

            self
        }

        #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
        {
            let mut this = self;
            let children = &mut this.children;

            match children {
                ElementChildren::Empty => {
                    *children = ElementChildren::Children(vec![child]);
                }
                ElementChildren::Children(ref mut children) => {
                    children.push(child);
                }
                _ => {
                    crate::debug_warn!(
                        "Don’t call .child() on an HtmlElement if you’ve \
                         already called .inner_html() or \
                         HtmlElement::from_chunks()."
                    );
                }
            }

            this
        }
    }

    /// Sets the inner HTML of this element from the provided
    /// string slice.
    ///
    /// # Security
    /// Be very careful when using this method. Always remember to
    /// sanitize the input to avoid a cross-site scripting (XSS)
    /// vulnerability.
    #[inline(always)]
    pub fn inner_html(self, html: impl Into<Cow<'static, str>>) -> Self {
        let html = html.into();

        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        {
            self.element.as_ref().set_inner_html(&html);

            self
        }

        #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
        {
            let mut this = self;

            this.children = ElementChildren::InnerHtml(html);

            this
        }
    }
}

impl<El: ElementDescriptor> IntoView for HtmlElement<El> {
    #[cfg_attr(any(debug_assertions, feature = "ssr"), instrument(level = "trace", name = "<HtmlElement />", skip_all, fields(tag = %self.element.name())))]
    #[cfg_attr(all(target_arch = "wasm32", feature = "web"), inline(always))]
    fn into_view(self, _: Scope) -> View {
        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        {
            View::Element(Element::new(self.element))
        }
        #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
        {
            let Self {
                element,
                mut attrs,
                children,
                #[cfg(debug_assertions)]
                view_marker,
                ..
            } = self;

            let id = *element.hydration_id();

            let mut element = Element::new(element);
            let children = children;

            if attrs.iter_mut().any(|(name, _)| name == "id") {
                attrs.push(("leptos-hk".into(), format!("_{id}").into()));
            } else {
                attrs.push(("id".into(), format!("_{id}").into()));
            }

            element.attrs = attrs;
            element.children = children;

            #[cfg(debug_assertions)]
            {
                element.view_marker = view_marker;
            }

            View::Element(element)
        }
    }
}

impl<El: ElementDescriptor, const N: usize> IntoView for [HtmlElement<El>; N] {
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(level = "trace", name = "[HtmlElement; N]", skip_all)
    )]
    fn into_view(self, cx: Scope) -> View {
        Fragment::new(self.into_iter().map(|el| el.into_view(cx)).collect())
            .into_view(cx)
    }
}

/// Creates any custom element, such as `<my-element>`.
pub fn custom<El: ElementDescriptor>(cx: Scope, el: El) -> HtmlElement<Custom> {
    HtmlElement::new(
        cx,
        Custom {
            name: el.name(),
            #[cfg(all(target_arch = "wasm32", feature = "web"))]
            element: el.as_ref().clone(),
            #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
            id: *el.hydration_id(),
        },
    )
}

/// Creates a text node.
#[inline(always)]
pub fn text(text: impl Into<Cow<'static, str>>) -> Text {
    Text::new(text.into())
}

macro_rules! generate_html_tags {
  ($(
    #[$meta:meta]
    $(#[$void:ident])?
    $tag:ident $([$trailing_:pat])? $el_type:ident
  ),* $(,)?) => {
    paste::paste! {
      $(
        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        thread_local! {
          static [<$tag:upper>]: LazyCell<web_sys::HtmlElement> = LazyCell::new(|| {
            crate::document()
              .create_element(stringify!($tag))
              .unwrap()
              .unchecked_into()
          });
        }

        #[derive(Clone, Debug)]
        #[$meta]
        pub struct [<$tag:camel $($trailing_)?>] {
          #[cfg(all(target_arch = "wasm32", feature = "web"))]
          element: web_sys::HtmlElement,
          #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
          id: HydrationKey,
        }

        impl Default for [<$tag:camel $($trailing_)?>] {
          fn default() -> Self {
            let id = HydrationCtx::id();

            #[cfg(all(target_arch = "wasm32", feature = "web"))]
            let element = create_leptos_element(
              &stringify!([<$tag:upper>]),
              id,
              || {
                [<$tag:upper>]
                .with(|el|
                  el.clone_node()
                    .unwrap()
                    .unchecked_into()
                )
              }
            );

            Self {
              #[cfg(all(target_arch = "wasm32", feature = "web"))]
              element,
              #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
              id
            }
          }
        }

        impl std::ops::Deref for [<$tag:camel $($trailing_)?>] {
          type Target = web_sys::$el_type;

          #[inline(always)]
          fn deref(&self) -> &Self::Target {
            #[cfg(all(target_arch = "wasm32", feature = "web"))]
            {
              use wasm_bindgen::JsCast;
              return &self.element.unchecked_ref();
            }

            #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
            unimplemented!("{HTML_ELEMENT_DEREF_UNIMPLEMENTED_MSG}");
          }
        }

        impl std::convert::AsRef<web_sys::HtmlElement> for [<$tag:camel $($trailing_)?>] {
          #[inline(always)]
          fn as_ref(&self) -> &web_sys::HtmlElement {
            #[cfg(all(target_arch = "wasm32", feature = "web"))]
            return &self.element;

            #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
            unimplemented!("{HTML_ELEMENT_DEREF_UNIMPLEMENTED_MSG}");
          }
        }

        impl ElementDescriptor for [<$tag:camel $($trailing_)?>] {
          #[inline(always)]
          fn name(&self) -> Cow<'static, str> {
            stringify!($tag).into()
          }

          #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
          #[inline(always)]
          fn hydration_id(&self) -> &HydrationKey {
            &self.id
          }

          generate_html_tags! { @void $($void)? }
        }

        #[$meta]
      #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        instrument(
          level = "trace",
          name = "HtmlElement",
          skip_all,
          fields(
            tag = %format!("<{}/>", stringify!($tag))
          )
        )
      )]
        pub fn $tag(cx: Scope) -> HtmlElement<[<$tag:camel $($trailing_)?>]> {
          HtmlElement::new(cx, [<$tag:camel $($trailing_)?>]::default())
        }
      )*
    }
  };
  (@void) => {};
  (@void void) => {
    #[inline(always)]
    fn is_void(&self) -> bool {
      true
    }
  }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
fn create_leptos_element(
    tag: &str,
    id: crate::HydrationKey,
    clone_element: fn() -> web_sys::HtmlElement,
) -> web_sys::HtmlElement {
    #[cfg(not(debug_assertions))]
    {
        _ = tag;
    }

    if HydrationCtx::is_hydrating() {
        if let Some(el) = crate::document().get_element_by_id(&format!("_{id}"))
        {
            #[cfg(debug_assertions)]
            assert_eq!(
                &el.node_name().to_ascii_uppercase(),
                tag,
                "SSR and CSR elements have the same hydration key but \
                different node kinds. Check out the docs for information \
                about this kind of hydration bug: https://leptos-rs.github.io/leptos/ssr/24_hydration_bugs.html"
            );

            el.remove_attribute("id").unwrap();

            el.unchecked_into()
        } else if let Ok(Some(el)) =
            crate::document().query_selector(&format!("[leptos-hk=_{id}]"))
        {
            #[cfg(debug_assertions)]
            assert_eq!(
                el.node_name().to_ascii_uppercase(),
                tag,
                "SSR and CSR elements have the same hydration key but \
                different node kinds. Check out the docs for information \
                about this kind of hydration bug: https://leptos-rs.github.io/leptos/ssr/24_hydration_bugs.html"
            );

            el.remove_attribute("leptos-hk").unwrap();

            el.unchecked_into()
        } else {
            if !is_meta_tag() {
                crate::warn!(
                    "element with id {id} not found, ignoring it for hydration"
                );
            }

            clone_element()
        }
    } else {
        clone_element()
    }
}

#[cfg(all(debug_assertions, target_arch = "wasm32", feature = "web"))]
fn warn_on_ambiguous_a(parent: &web_sys::Element, child: &View) {
    if let View::Element(el) = &child {
        if (el.name == "a"
            || el.name == "script"
            || el.name == "style"
            || el.name == "title")
            && parent.namespace_uri() != el.element.namespace_uri()
        {
            crate::warn!(
                "Warning: you are appending an SVG element to an HTML \
                 element, or an HTML element to an SVG. Typically, this \
                 occurs when you create an <a/> or <script/> with the `view` \
                 macro and append it to an SVG, but the framework assumed it \
                 was HTML when you created it. To specify that it is an SVG \
                 element, use <svg::{{tag name}}/> in the view macro."
            )
        }
    }
}

generate_html_tags![
  // ==========================
  //        Main root
  // ==========================
  /// The `<html>` HTML element represents the root (top-level element) of an HTML document, so it is also referred to as the root element. All other elements must be descendants of this element.
  html HtmlHtmlElement,
  // ==========================
  //     Document Metadata
  // ==========================
  /// The `<base>` HTML element specifies the base URL to use for all relative URLs in a document. There can be only one `<base>` element in a document.
  #[void]
  base HtmlBaseElement,
  ///	The `<head>` HTML element contains machine-readable information (metadata) about the document, like its title, scripts, and style sheets.
  head HtmlHeadElement,
  ///	The `<link>` HTML element specifies relationships between the current document and an external resource. This element is most commonly used to link to CSS, but is also used to establish site icons (both "favicon" style icons and icons for the home screen and apps on mobile devices) among other things.
  #[void]
  link HtmlLinkElement,
  ///	The `<meta>` HTML element represents Metadata that cannot be represented by other HTML meta-related elements, like base, link, script, style or title.
  #[void]
  meta HtmlMetaElement,
  ///	The `<style>` HTML element contains style information for a document, or part of a document. It contains CSS, which is applied to the contents of the document containing the `<style>` element.
  style HtmlStyleElement,
  ///	The `<title>` HTML element defines the document's title that is shown in a Browser's title bar or a page's tab. It only contains text; tags within the element are ignored.
  title HtmlTitleElement,
  // ==========================
  //     Sectioning Root
  // ==========================
  /// The `<body>` HTML element represents the content of an HTML document. There can be only one `<body>` element in a document.
  body HtmlBodyElement,
  // ==========================
  //     Content Sectioning
  // ==========================
  /// The `<address>` HTML element indicates that the enclosed HTML provides contact information for a person or people, or for an organization.
  address HtmlElement,
  /// The `<article>` HTML element represents a self-contained composition in a document, page, application, or site, which is intended to be independently distributable or reusable (e.g., in syndication). Examples include: a forum post, a magazine or newspaper article, or a blog entry, a product card, a user-submitted comment, an interactive widget or gadget, or any other independent item of content.
  article HtmlElement,
  /// The `<aside>` HTML element represents a portion of a document whose content is only indirectly related to the document's main content. Asides are frequently presented as sidebars or call-out boxes.
  aside HtmlElement,
  /// The `<footer>` HTML element represents a footer for its nearest sectioning content or sectioning root element. A `<footer>` typically contains information about the author of the section, copyright data or links to related documents.
  footer HtmlElement,
  /// The `<header>` HTML element represents introductory content, typically a group of introductory or navigational aids. It may contain some heading elements but also a logo, a search form, an author name, and other elements.
  header HtmlElement,
  /// The `<hgroup>` HTML element represents a heading and related content. It groups a single `<h1>–<h6>` element with one or more `<p>`.
  hgroup HtmlElement,
  /// The `<h1>` to `<h6>` HTML elements represent six levels of section headings. `<h1>` is the highest section level and `<h6>` is the lowest.
  h1 HtmlHeadingElement,
  /// The `<h1>` to `<h6>` HTML elements represent six levels of section headings. `<h1>` is the highest section level and `<h6>` is the lowest.
  h2 HtmlHeadingElement,
  /// The `<h1>` to `<h6>` HTML elements represent six levels of section headings. `<h1>` is the highest section level and `<h6>` is the lowest.
  h3 HtmlHeadingElement,
  /// The `<h1>` to `<h6>` HTML elements represent six levels of section headings. `<h1>` is the highest section level and `<h6>` is the lowest.
  h4 HtmlHeadingElement,
  /// The `<h1>` to `<h6>` HTML elements represent six levels of section headings. `<h1>` is the highest section level and `<h6>` is the lowest.
  h5 HtmlHeadingElement,
  /// The `<h1>` to `<h6>` HTML elements represent six levels of section headings. `<h1>` is the highest section level and `<h6>` is the lowest.
  h6 HtmlHeadingElement,
  /// The `<main>` HTML element represents the dominant content of the body of a document. The main content area consists of content that is directly related to or expands upon the central topic of a document, or the central functionality of an application.
  main HtmlElement,
  /// The `<nav>` HTML element represents a section of a page whose purpose is to provide navigation links, either within the current document or to other documents. Common examples of navigation sections are menus, tables of contents, and indexes.
  nav HtmlElement,
  /// The `<section>` HTML element represents a generic standalone section of a document, which doesn't have a more specific semantic element to represent it. Sections should always have a heading, with very few exceptions.
  section HtmlElement,
  // ==========================
  //      Text Content
  // ==========================
  /// The `<blockquote>` HTML element indicates that the enclosed text is an extended quotation. Usually, this is rendered visually by indentation (see Notes for how to change it). A URL for the source of the quotation may be given using the cite attribute, while a text representation of the source can be given using the cite element.
  blockquote HtmlQuoteElement,
  /// The `<dd>` HTML element provides the description, definition, or value for the preceding term (dt) in a description list (dl).
  dd HtmlElement,
  /// The `<div>` HTML element is the generic container for flow content. It has no effect on the content or layout until styled in some way using CSS (e.g. styling is directly applied to it, or some kind of layout model like Flexbox is applied to its parent element).
  div HtmlDivElement,
  /// The `<dl>` HTML element represents a description list. The element encloses a list of groups of terms (specified using the dt element) and descriptions (provided by dd elements). Common uses for this element are to implement a glossary or to display metadata (a list of key-value pairs).
  dl HtmlDListElement,
  /// The `<dt>` HTML element specifies a term in a description or definition list, and as such must be used inside a dl element. It is usually followed by a dd element; however, multiple `<dt>` elements in a row indicate several terms that are all defined by the immediate next dd element.
  dt HtmlElement,
  /// The `<figcaption>` HTML element represents a caption or legend describing the rest of the contents of its parent figure element.
  figcaption HtmlElement,
  /// The `<figure>` HTML element represents self-contained content, potentially with an optional caption, which is specified using the figcaption element. The figure, its caption, and its contents are referenced as a single unit.
  figure HtmlElement,
  /// The `<hr>` HTML element represents a thematic break between paragraph-level elements: for example, a change of scene in a story, or a shift of topic within a section.
  #[void]
  hr HtmlHrElement,
  /// The `<li>` HTML element is used to represent an item in a list. It must be contained in a parent element: an ordered list (ol), an unordered list (ul), or a menu (menu). In menus and unordered lists, list items are usually displayed using bullet points. In ordered lists, they are usually displayed with an ascending counter on the left, such as a number or letter.
  li HtmlLiElement,
  /// The `<ol>` HTML element represents an ordered list of items — typically rendered as a numbered list.
  ol HtmlOListElement,
  /// The `<p>` HTML element represents a paragraph. Paragraphs are usually represented in visual media as blocks of text separated from adjacent blocks by blank lines and/or first-line indentation, but HTML paragraphs can be any structural grouping of related content, such as images or form fields.
  p HtmlParagraphElement,
  /// The `<pre>` HTML element represents preformatted text which is to be presented exactly as written in the HTML file. The text is typically rendered using a non-proportional, or "monospaced, font. Whitespace inside this element is displayed as written.
  pre HtmlPreElement,
  /// The `<ul>` HTML element represents an unordered list of items, typically rendered as a bulleted list.
  ul HtmlUListElement,
  // ==========================
  //    Inline Text Semantics
  // ==========================
  /// The `<a>` HTML element (or anchor element), with its href attribute, creates a hyperlink to web pages, files, email addresses, locations in the same page, or anything else a URL can address.
  a HtmlAnchorElement,
  /// The `<abbr>` HTML element represents an abbreviation or acronym; the optional title attribute can provide an expansion or description for the abbreviation. If present, title must contain this full description and nothing else.
  abbr HtmlElement,
  /// The `<b>` HTML element is used to draw the reader's attention to the element's contents, which are not otherwise granted special importance. This was formerly known as the Boldface element, and most browsers still draw the text in boldface. However, you should not use `<b>` for styling text; instead, you should use the CSS font-weight property to create boldface text, or the strong element to indicate that text is of special importance.
  b HtmlElement,
  /// The `<bdi>` HTML element tells the browser's bidirectional algorithm to treat the text it contains in isolation from its surrounding text. It's particularly useful when a website dynamically inserts some text and doesn't know the directionality of the text being inserted.
  bdi HtmlElement,
  /// The `<bdo>` HTML element overrides the current directionality of text, so that the text within is rendered in a different direction.
  bdo HtmlElement,
  /// The `<br>` HTML element produces a line break in text (carriage-return). It is useful for writing a poem or an address, where the division of lines is significant.
  #[void]
  br HtmlBrElement,
  /// The `<cite>` HTML element is used to describe a reference to a cited creative work, and must include the title of that work. The reference may be in an abbreviated form according to context-appropriate conventions related to citation metadata.
  cite HtmlElement,
  /// The `<code>` HTML element displays its contents styled in a fashion intended to indicate that the text is a short fragment of computer code. By default, the content text is displayed using the user agent default monospace font.
  code HtmlElement,
  /// The `<data>` HTML element links a given piece of content with a machine-readable translation. If the content is time- or date-related, the time element must be used.
  data HtmlDataElement,
  /// The `<dfn>` HTML element is used to indicate the term being defined within the context of a definition phrase or sentence. The p element, the dt/dd pairing, or the section element which is the nearest ancestor of the `<dfn>` is considered to be the definition of the term.
  dfn HtmlElement,
  /// The `<em>` HTML element marks text that has stress emphasis. The `<em>` element can be nested, with each level of nesting indicating a greater degree of emphasis.
  em HtmlElement,
  /// The `<i>` HTML element represents a range of text that is set off from the normal text for some reason, such as idiomatic text, technical terms, taxonomical designations, among others. Historically, these have been presented using italicized type, which is the original source of the `<i>` naming of this element.
  i HtmlElement,
  /// The `<kbd>` HTML element represents a span of inline text denoting textual user input from a keyboard, voice input, or any other text entry device. By convention, the user agent defaults to rendering the contents of a `<kbd>` element using its default monospace font, although this is not mandated by the HTML standard.
  kbd HtmlElement,
  /// The `<mark>` HTML element represents text which is marked or highlighted for reference or notation purposes, due to the marked passage's relevance or importance in the enclosing context.
  mark HtmlElement,
  /// The `<q>` HTML element indicates that the enclosed text is a short inline quotation. Most modern browsers implement this by surrounding the text in quotation marks. This element is intended for short quotations that don't require paragraph breaks; for long quotations use the blockquote element.
  q HtmlQuoteElement,
  /// The `<rp>` HTML element is used to provide fall-back parentheses for browsers that do not support display of ruby annotations using the ruby element. One `<rp>` element should enclose each of the opening and closing parentheses that wrap the rt element that contains the annotation's text.
  rp HtmlElement,
  /// The `<rt>` HTML element specifies the ruby text component of a ruby annotation, which is used to provide pronunciation, translation, or transliteration information for East Asian typography. The `<rt>` element must always be contained within a ruby element.
  rt HtmlElement,
  /// The `<ruby>` HTML element represents small annotations that are rendered above, below, or next to base text, usually used for showing the pronunciation of East Asian characters. It can also be used for annotating other kinds of text, but this usage is less common.
  ruby HtmlElement,
  /// The `<s>` HTML element renders text with a strikethrough, or a line through it. Use the `<s>` element to represent things that are no longer relevant or no longer accurate. However, `<s>` is not appropriate when indicating document edits; for that, use the del and ins elements, as appropriate.
  s HtmlElement,
  /// The `<samp>` HTML element is used to enclose inline text which represents sample (or quoted) output from a computer program. Its contents are typically rendered using the browser's default monospaced font (such as Courier or Lucida Console).
  samp HtmlElement,
  /// The `<small>` HTML element represents side-comments and small print, like copyright and legal text, independent of its styled presentation. By default, it renders text within it one font-size smaller, such as from small to x-small.
  small HtmlElement,
  /// The `<span>` HTML element is a generic inline container for phrasing content, which does not inherently represent anything. It can be used to group elements for styling purposes (using the class or id attributes), or because they share attribute values, such as lang. It should be used only when no other semantic element is appropriate. `<span>` is very much like a div element, but div is a block-level element whereas a `<span>` is an inline element.
  span HtmlSpanElement,
  /// The `<strong>` HTML element indicates that its contents have strong importance, seriousness, or urgency. Browsers typically render the contents in bold type.
  strong HtmlElement,
  /// The `<sub>` HTML element specifies inline text which should be displayed as subscript for solely typographical reasons. Subscripts are typically rendered with a lowered baseline using smaller text.
  sub HtmlElement,
  /// The `<sup>` HTML element specifies inline text which is to be displayed as superscript for solely typographical reasons. Superscripts are usually rendered with a raised baseline using smaller text.
  sup HtmlElement,
  /// The `<time>` HTML element represents a specific period in time. It may include the datetime attribute to translate dates into machine-readable format, allowing for better search engine results or custom features such as reminders.
  time HtmlTimeElement,
  /// The `<u>` HTML element represents a span of inline text which should be rendered in a way that indicates that it has a non-textual annotation. This is rendered by default as a simple solid underline, but may be altered using CSS.
  u HtmlElement,
  /// The `<var>` HTML element represents the name of a variable in a mathematical expression or a programming context. It's typically presented using an italicized version of the current typeface, although that behavior is browser-dependent.
  var HtmlElement,
  /// The `<wbr>` HTML element represents a word break opportunity—a position within text where the browser may optionally break a line, though its line-breaking rules would not otherwise create a break at that location.
  #[void]
  wbr HtmlElement,
  // ==========================
  //   Image and multimedia
  // ==========================
  /// The `<area>` HTML element defines an area inside an image map that has predefined clickable areas. An image map allows geometric areas on an image to be associated with Hyperlink.
  #[void]
  area HtmlAreaElement,
  /// The `<audio>` HTML element is used to embed sound content in documents. It may contain one or more audio sources, represented using the src attribute or the source element: the browser will choose the most suitable one. It can also be the destination for streamed media, using a MediaStream.
  audio HtmlAudioElement,
  /// The `<img>` HTML element embeds an image into the document.
  #[void]
  img HtmlImageElement,
  /// The `<map>` HTML element is used with area elements to define an image map (a clickable link area).
  map HtmlMapElement,
  /// The `<track>` HTML element is used as a child of the media elements, audio and video. It lets you specify timed text tracks (or time-based data), for example to automatically handle subtitles. The tracks are formatted in WebVTT format (.vtt files) — Web Video Text Tracks.
  #[void]
  track HtmlTrackElement,
  /// The `<video>` HTML element embeds a media player which supports video playback into the document. You can use `<video>` for audio content as well, but the audio element may provide a more appropriate user experience.
  video HtmlVideoElement,
  // ==========================
  //     Embedded Content
  // ==========================
  /// The `<embed>` HTML element embeds external content at the specified point in the document. This content is provided by an external application or other source of interactive content such as a browser plug-in.
  #[void]
  embed HtmlEmbedElement,
  /// The `<iframe>` HTML element represents a nested browsing context, embedding another HTML page into the current one.
  iframe HtmlIFrameElement,
  /// The `<object>` HTML element represents an external resource, which can be treated as an image, a nested browsing context, or a resource to be handled by a plugin.
  object HtmlObjectElement,
  /// The `<param>` HTML element defines parameters for an object element.
  #[void]
  param HtmlParamElement,
  /// The `<picture>` HTML element contains zero or more source elements and one img element to offer alternative versions of an image for different display/device scenarios.
  picture HtmlPictureElement,
  /// The `<portal>` HTML element enables the embedding of another HTML page into the current one for the purposes of allowing smoother navigation into new pages.
  portal HtmlElement,
  /// The `<source>` HTML element specifies multiple media resources for the picture, the audio element, or the video element. It is an empty element, meaning that it has no content and does not have a closing tag. It is commonly used to offer the same media content in multiple file formats in order to provide compatibility with a broad range of browsers given their differing support for image file formats and media file formats.
  #[void]
  source HtmlSourceElement,
  // ==========================
  //      SVG and MathML
  // ==========================
  /// The svg element is a container that defines a new coordinate system and viewport. It is used as the outermost element of SVG documents, but it can also be used to embed an SVG fragment inside an SVG or HTML document.
  svg SvgElement,
  /// The top-level element in MathML is `<math>.` Every valid MathML instance must be wrapped in `<math>` tags. In addition you must not nest a second `<math>` element in another, but you can have an arbitrary number of other child elements in it.
  math HtmlElement,
  // ==========================
  //         Scripting
  // ==========================
  /// Use the HTML `<canvas>` element with either the canvas scripting API or the WebGL API to draw graphics and animations.
  canvas HtmlCanvasElement,
  /// The `<noscript>` HTML element defines a section of HTML to be inserted if a script type on the page is unsupported or if scripting is currently turned off in the browser.
  noscript HtmlElement,
  /// The `<script>` HTML element is used to embed executable code or data; this is typically used to embed or refer to JavaScript code. The `<script>` element can also be used with other languages, such as WebGL's GLSL shader programming language and JSON.
  script HtmlScriptElement,
  // ==========================
  //     Demarcating Edits
  // ==========================
  /// The `<del>` HTML element represents a range of text that has been deleted from a document. This can be used when rendering "track changes" or source code diff information, for example. The ins element can be used for the opposite purpose: to indicate text that has been added to the document.
  del HtmlModElement,
  /// The `<ins>` HTML element represents a range of text that has been added to a document. You can use the del element to similarly represent a range of text that has been deleted from the document.
  ins HtmlElement,
  // ==========================
  //     Table Content
  // ==========================
  /// The `<caption>` HTML element specifies the caption (or title) of a table.
  caption HtmlTableCaptionElement,
  /// The `<col>` HTML element defines a column within a table and is used for defining common semantics on all common cells. It is generally found within a colgroup element.
  #[void]
  col HtmlTableColElement,
  /// The `<colgroup>` HTML element defines a group of columns within a table.
  colgroup HtmlTableColElement,
  /// The `<table>` HTML element represents tabular data — that is, information presented in a two-dimensional table comprised of rows and columns of cells containing data.
  table HtmlTableElement,
  /// The `<tbody>` HTML element encapsulates a set of table rows (tr elements), indicating that they comprise the body of the table (table).
  tbody HtmlTableSectionElement,
  /// The `<td>` HTML element defines a cell of a table that contains data. It participates in the table model.
  td HtmlTableCellElement,
  /// The `<tfoot>` HTML element defines a set of rows summarizing the columns of the table.
  tfoot HtmlTableSectionElement,
  /// The `<th>` HTML element defines a cell as header of a group of table cells. The exact nature of this group is defined by the scope and headers attributes.
  th HtmlTableCellElement,
  /// The `<thead>` HTML element defines a set of rows defining the head of the columns of the table.
  thead HtmlTableSectionElement,
  /// The `<tr>` HTML element defines a row of cells in a table. The row's cells can then be established using a mix of td (data cell) and th (header cell) elements.
  tr HtmlTableRowElement,
  // ==========================
  //          Forms
  // ==========================
  /// The `<button>` HTML element represents a clickable button, used to submit forms or anywhere in a document for accessible, standard button functionality.
  button HtmlButtonElement,
  /// The `<datalist>` HTML element contains a set of option elements that represent the permissible or recommended options available to choose from within other controls.
  datalist HtmlDataListElement,
  /// The `<fieldset>` HTML element is used to group several controls as well as labels (label) within a web form.
  fieldset HtmlFieldSetElement,
  /// The `<form>` HTML element represents a document section containing interactive controls for submitting information.
  form HtmlFormElement,
  /// The `<input>` HTML element is used to create interactive controls for web-based forms in order to accept data from the user; a wide variety of types of input data and control widgets are available, depending on the device and user agent. The `<input>` element is one of the most powerful and complex in all of HTML due to the sheer number of combinations of input types and attributes.
  #[void]
  input HtmlInputElement,
  /// The `<label>` HTML element represents a caption for an item in a user interface.
  label HtmlLabelElement,
  /// The `<legend>` HTML element represents a caption for the content of its parent fieldset.
  legend HtmlLegendElement,
  /// The `<meter>` HTML element represents either a scalar value within a known range or a fractional value.
  meter HtmlMeterElement,
  /// The `<optgroup>` HTML element creates a grouping of options within a select element.
  optgroup HtmlOptGroupElement,
  /// The `<option>` HTML element is used to define an item contained in a select, an optgroup, or a datalist element. As such, `<option>` can represent menu items in popups and other lists of items in an HTML document.
  option[_] HtmlOptionElement,
  /// The `<output>` HTML element is a container element into which a site or app can inject the results of a calculation or the outcome of a user action.
  output HtmlOutputElement,
  /// The `<progress>` HTML element displays an indicator showing the completion progress of a task, typically displayed as a progress bar.
  progress HtmlProgressElement,
  /// The `<select>` HTML element represents a control that provides a menu of options:
  select HtmlSelectElement,
  /// The `<textarea>` HTML element represents a multi-line plain-text editing control, useful when you want to allow users to enter a sizeable amount of free-form text, for example a comment on a review or feedback form.
  textarea HtmlTextAreaElement,
  // ==========================
  //    Interactive elements
  // ==========================
  /// The `<details>` HTML element creates a disclosure widget in which information is visible only when the widget is toggled into an "open" state. A summary or label must be provided using the summary element.
  details HtmlDetailsElement,
  /// The `<dialog>` HTML element represents a dialog box or other interactive component, such as a dismissible alert, inspector, or subwindow.
  dialog HtmlDialogElement,
  /// The `<menu>` HTML element is a semantic alternative to ul. It represents an unordered list of items (represented by li elements), each of these represent a link or other command that the user can activate.
  menu HtmlMenuElement,
  /// The `<summary>` HTML element specifies a summary, caption, or legend for a details element's disclosure box. Clicking the `<summary>` element toggles the state of the parent `<details>` element open and closed.
  summary HtmlElement,
  // ==========================
  //      Web Components
  // ==========================
  /// The `<slot>` HTML element—part of the Web Components technology suite—is a placeholder inside a web component that you can fill with your own markup, which lets you create separate DOM trees and present them together.
  slot HtmlSlotElement,
  /// The `<template>` HTML element is a mechanism for holding HTML that is not to be rendered immediately when a page is loaded but may be instantiated subsequently during runtime using JavaScript.
  template HtmlTemplateElement,
];
