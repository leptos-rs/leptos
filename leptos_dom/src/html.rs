#[cfg(all(target_arch = "wasm32", feature = "web"))]
use crate::events::*;
use crate::{
  components::DynChild,
  ev::EventDescriptor,
  hydration::HydrationCtx,
  macro_helpers::{
    attribute_expression, class_expression, property_expression, Attribute,
    Child, Class, IntoAttribute, IntoChild, IntoClass, IntoProperty, Property,
  },
  Element, Fragment, IntoView, NodeRef, Text, View,
};
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use crate::{mount_child, MountKind};
use cfg_if::cfg_if;
use leptos_reactive::{create_effect, create_render_effect, Scope};
use smallvec::{smallvec, SmallVec};
use std::{
  borrow::Cow,
  cell::{LazyCell, OnceCell},
  fmt,
  ops::Deref,
};
use wasm_bindgen::{convert::FromWasmAbi, intern, JsCast, JsValue};

/// Trait alias for the trait bounts on [`IntoElement`].
#[cfg(all(target_arch = "wasm32", feature = "web"))]
pub trait IntoElementBounds:
  fmt::Debug + Deref<Target = web_sys::HtmlElement>
{
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
impl<El> IntoElementBounds for El where
  El: fmt::Debug + Deref<Target = web_sys::HtmlElement>
{
}

/// Trait alias for the trait bounts on [`IntoElement`].
#[cfg(not(all(target_arch = "wasm32", feature = "web")))]
pub trait IntoElementBounds: fmt::Debug {}

#[cfg(not(all(target_arch = "wasm32", feature = "web")))]
impl<El> IntoElementBounds for El where El: fmt::Debug {}

/// Trait which allows creating an element tag.
pub trait IntoElement: IntoElementBounds {
  /// The name of the element, i.e., `div`, `p`, `custom-element`.
  fn name(&self) -> Cow<'static, str>;

  /// Get a reference to the underlying [`web_sys::HtmlElement`].
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  fn get_element(&self) -> &web_sys::HtmlElement;

  /// Determains if the tag is void, i.e., `<input>` and `<br>`.
  fn is_void(&self) -> bool {
    false
  }

  /// A unique `id` that should be generated for each new instance of
  /// this element, and be consitant for both SSR and CSR.
  #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
  fn hydration_id(&self) -> usize;
}

/// Represents potentially any element.
#[derive(Clone, Debug)]
#[cfg_attr(all(target_arch = "wasm32", feature = "web"), derive(educe::Educe))]
#[cfg_attr(all(target_arch = "wasm32", feature = "web"), educe(Deref))]
pub struct AnyElement {
  name: Cow<'static, str>,
  #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
  dynamic: bool,
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  #[educe(Deref)]
  element: web_sys::HtmlElement,
  is_void: bool,
  #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
  id: usize,
}

impl IntoElement for AnyElement {
  fn name(&self) -> Cow<'static, str> {
    self.name.clone()
  }

  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  fn get_element(&self) -> &web_sys::HtmlElement {
    &self.element
  }

  fn is_void(&self) -> bool {
    self.is_void
  }

  #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
  fn hydration_id(&self) -> usize {
    self.id
  }
}

/// Represents a custom HTML element, such as `<my-element>`.
#[derive(Clone, Debug)]
pub struct Custom {
  name: Cow<'static, str>,
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  element: web_sys::HtmlElement,
  #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
  id: usize,
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
impl std::ops::Deref for Custom {
  type Target = web_sys::HtmlElement;

  fn deref(&self) -> &Self::Target {
    &self.element
  }
}

impl IntoElement for Custom {
  fn name(&self) -> Cow<'static, str> {
    self.name.clone()
  }

  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  fn get_element(&self) -> &web_sys::HtmlElement {
    &self.element
  }

  #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
  fn hydration_id(&self) -> usize {
    self.id
  }
}

cfg_if! {
  if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
    /// Represents an HTML element.
    #[derive(educe::Educe)]
    #[educe(Debug, Deref)]
    pub struct HtmlElement<El: IntoElement> {
      cx: Scope,
      #[educe(Deref)]
      element: El,
    }
  // Server needs to build a virtualized DOM tree
  } else {
    /// Represents an HTML element.
    #[derive(educe::Educe)]
    #[educe(Debug)]
    pub struct HtmlElement<El: IntoElement> {
      pub(crate) cx: Scope,
      pub(crate) element: El,
      pub(crate) id: OnceCell<Cow<'static, str>>,
      pub(crate) dynamic: bool,
      #[educe(Debug(ignore))]
      pub(crate) attrs: SmallVec<[(Cow<'static, str>, Cow<'static, str>); 4]>,
      #[educe(Debug(ignore))]
      #[allow(clippy::type_complexity)]
      pub(crate) children: SmallVec<[View; 4]>,
    }
  }
}

impl<El: IntoElement> HtmlElement<El> {
  fn new(cx: Scope, element: El) -> Self {
    cfg_if! {
      if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
        Self {
          cx,
          element,
        }
      } else {
        Self {
          cx,
          id: Default::default(),
          dynamic: false,
          attrs: smallvec![],
          children: smallvec![],
          element,
        }
      }
    }
  }

  /// Converts this element into [`HtmlElement<AnyElement>`].
  pub fn into_any(self) -> HtmlElement<AnyElement> {
    cfg_if! {
      if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
        let Self {
          cx,
          element,
        } = self;

        HtmlElement {
          cx,
          element: AnyElement {
            name: element.name(),
            element: element.get_element().clone(),
            is_void: element.is_void(),
          },
        }
      } else {
        let Self {
          cx,
          id,
          dynamic,
          attrs,
          children,
          element,
        } = self;

        HtmlElement {
          cx,
          id,
          dynamic,
          attrs,
          children,
          element: AnyElement {
            name: element.name(),
            dynamic,
            is_void: element.is_void(),
            id: element.hydration_id(),
          },
        }
      }
    }
  }

  /// Adds an `id` to the element.
  #[track_caller]
  pub fn id(mut self, id: impl Into<Cow<'static, str>>) -> Self {
    let id = id.into();

    cfg_if! {
      if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
        self
          .element
          .get_element()
          .set_attribute(intern("id"), &id)
          .unwrap();
      }
      else {
        self.attrs.push(("id".into(), id));
      }
    }

    self
  }

  /// Binds the element reference to [`NodeRef`].
  pub fn node_ref(self, node_ref: &NodeRef) -> Self {
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    node_ref.load(&self);

    self
  }

  #[doc(hidden)]
  #[track_caller]
  pub fn attr(
    mut self,
    name: impl Into<Cow<'static, str>>,
    attr: impl IntoAttribute,
  ) -> Self {
    let name = name.into();
    cfg_if! {
      if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
        let el = self.element.get_element();
        let value = attr.into_attribute(self.cx);
        match value {
          Attribute::Fn(cx, f) => {
              let el = el.clone();
              create_render_effect(cx, move |old| {
                  let new = f();
                  if old.as_ref() != Some(&new) {
                      attribute_expression(&el, &name, new.clone());
                  }
                  new
              });
          }
          _ => attribute_expression(el, &name, value),
        };
        self
      }
      else {
        let mut attr = attr.into_attribute(self.cx);
        while let Attribute::Fn(_, f) = attr {
          self.dynamic = true;
          attr = f();
        }
        match attr {
          Attribute::String(value) => {
            self.attrs.push((name, value.into()));
            self
          },
          Attribute::Bool(include) => if include {
            self.attrs.push((name, "".into()));
            self
          } else {
            self
          },
          Attribute::Option(_, maybe) => if let Some(value) = maybe {
            self.attrs.push((name, value.into()));
            self
          } else {
            self
          }
          _ => unreachable!()
        }
      }
    }
  }

  /// Adds a class to an element.
  #[track_caller]
  pub fn class(
    mut self,
    name: impl Into<Cow<'static, str>>,
    class: impl IntoClass,
  ) -> Self {
    let name = name.into();
    cfg_if! {
    if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
      let el = self.element.get_element();
      let class_list = el.class_list();
      let value = class.into_class(self.cx);
      match value {
        Class::Fn(cx, f) => {
            create_render_effect(cx, move |old| {
              let new = f();
              if old.as_ref() != Some(&new) && (old.is_some() || new) {
                class_expression(&class_list, &name, new)
              }
              new
            });
        }
        Class::Value(value) => class_expression(&class_list, &name, value),
      };
      self
    }
    else {
      let mut class = class.into_class(self.cx);

      let include = match class {
        Class::Value(include) => include,
        Class::Fn(_, f) => {
          self.dynamic = true;
          f()
        }
      };

      if include {
        if let Some((_, ref mut value)) = self.attrs.iter_mut().find(|(name, _)| name == "class") {
          *value = format!("{value} {name}").into();
          self
        } else {
          self.attrs.push(("class".into(), name));
          self
        }
      } else {
        self
      }
    }
    }
  }

  /// Sets a property on an element.
  #[track_caller]
  pub fn prop(
    mut self,
    name: impl Into<Cow<'static, str>>,
    value: impl IntoProperty,
  ) -> Self {
    cfg_if! {
      if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
        let name = name.into();
        let value = value.into_property(self.cx);
        let el = self.element.get_element();
        match value {
          Property::Fn(cx, f) => {
            let el = el.clone();
            create_render_effect(cx, move |old| {
                let new = f();
                let prop_name = wasm_bindgen::intern(&name);
                if old.as_ref() != Some(&new) && !(old.is_none() && new == wasm_bindgen::JsValue::UNDEFINED) {
                    property_expression(&el, &prop_name, new.clone())
                }
                new
            });
          }
          Property::Value(value) => {
            let prop_name = wasm_bindgen::intern(&name);
            property_expression(&el, &prop_name, value)
          },
        };
        self
      }
      else {
        self.dynamic = true;
        self
      }
    }
  }

  /// Adds an event listener to this element.
  #[track_caller]
  pub fn on<E: EventDescriptor + 'static>(
    mut self,
    event: E,
    event_handler: impl FnMut(E::EventType) + 'static,
  ) -> Self {
    cfg_if! {
      if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
        let event_name = event.name();

        if event.bubbles() {
          add_event_listener(self.element.get_element(), event_name, event_handler);
        } else {
          add_event_listener_undelegated(self.element.get_element(), &event_name, event_handler);
        }

      } else {
        self.dynamic = true;
        _ = event;
        _ = event_handler;
      }
    }

    self
  }

  #[doc(hidden)]
  #[track_caller]
  pub fn child(mut self, child: impl IntoChild) -> Self {
    let child = child.into_child(self.cx).into_view(self.cx);
    cfg_if! {
      if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
        mount_child(MountKind::Append(self.element.get_element()), &child);
      }
      else {
        self.children.push(child);
      }
    }

    self
  }
}

impl<El: IntoElement> IntoView for HtmlElement<El> {
  #[cfg_attr(debug_assertions, instrument(level = "trace", name = "<HtmlElement />", skip_all, fields(tag = %self.element.name())))]
  fn into_view(self, cx: Scope) -> View {
    cfg_if! {
      if #[cfg(all(target_arch = "wasm32", feature = "web"))] {
        View::Element(Element::new(self.element))
      } else {
        let Self { element, mut attrs, children, dynamic, .. } = self;

        let id = element.hydration_id();

        let mut element = Element::new(element);
        let children = children;

        if !attrs.iter_mut().any(|(name, _)| name == "id") {
          attrs.push(("id".into(), format!("_{}", id).into()));
        }

        element.dynamic = dynamic;
        element.attrs = attrs;
        element.children.extend(children);

        View::Element(element)
      }
    }
  }
}

impl<El: IntoElement> IntoView for Vec<HtmlElement<El>> {
  #[cfg_attr(
    debug_assertions,
    instrument(level = "trace", name = "Vec<HtmlElement>", skip_all)
  )]
  fn into_view(self, cx: Scope) -> View {
    Fragment::new(self.into_iter().map(|el| el.into_view(cx)).collect())
      .into_view(cx)
  }
}

impl<El: IntoElement, const N: usize> IntoView for [HtmlElement<El>; N] {
  #[cfg_attr(
    debug_assertions,
    instrument(level = "trace", name = "[HtmlElement; N]", skip_all)
  )]
  fn into_view(self, cx: Scope) -> View {
    Fragment::new(self.into_iter().map(|el| el.into_view(cx)).collect())
      .into_view(cx)
  }
}

/// Creates any custom element, such as `<my-element>`.
pub fn custom<El: IntoElement>(cx: Scope, el: El) -> HtmlElement<Custom> {
  HtmlElement::new(
    cx,
    Custom {
      name: el.name(),
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      element: el.get_element().clone(),
      #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
      id: el.hydration_id(),
    },
  )
}

/// Creates a text node.
pub fn text(text: impl Into<Cow<'static, str>>) -> Text {
  Text::new(text.into())
}

macro_rules! generate_html_tags {
  ($(
    #[$meta:meta]
    $(#[$void:ident])?
    $tag:ident $([$trailing_:pat])?
  ),* $(,)?) => {
    paste::paste! {
      $(
        #[thread_local]
        static [<$tag:upper>]: LazyCell<web_sys::HtmlElement> = LazyCell::new(|| {
          crate::document()
            .create_element(stringify!($tag))
            .unwrap()
            .unchecked_into()
        });

        #[derive(Clone, Debug)]
        #[cfg_attr(all(target_arch = "wasm32", feature = "web"), derive(educe::Educe))]
        #[cfg_attr(all(target_arch = "wasm32", feature = "web"), educe(Deref))]
        #[$meta]
        pub struct [<$tag:camel $($trailing_)?>] {
          #[cfg(all(target_arch = "wasm32", feature = "web"))]
          #[educe(Deref)]
          element: web_sys::HtmlElement,
          #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
          id: usize,
        }

        impl Default for [<$tag:camel $($trailing_)?>] {
          fn default() -> Self {
            let id = HydrationCtx::id();

            #[cfg(all(target_arch = "wasm32", feature = "web"))]
            let element = if HydrationCtx::is_hydrating() {
              if let Some(el) = crate::document().get_element_by_id(
                &format!("_{id}")
              ) {
                #[cfg(debug_assertions)]
                assert_eq!(
                  el.node_name(),
                  stringify!([<$tag:upper>]),
                  "SSR and CSR elements have the same `TopoId` \
                    but different node kinds. This is either a \
                    discrepancy between SSR and CSR rendering
                    logic, which is considered a bug, or it \
                    can also be a leptos hydration issue."
                );

                el.unchecked_into()
              } else {
                panic!("SSR and CSR hydration mismatch, element id `_{id}` not found!");
              }
            } else {
              [<$tag:upper>].clone_node().unwrap().unchecked_into()
            };

            Self {
              #[cfg(all(target_arch = "wasm32", feature = "web"))]
              element,
              #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
              id
            }
          }
        }

        impl IntoElement for [<$tag:camel $($trailing_)?>] {
          fn name(&self) -> Cow<'static, str> {
            stringify!($tag).into()
          }

          #[cfg(all(target_arch = "wasm32", feature = "web"))]
          fn get_element(&self) -> &web_sys::HtmlElement {
            &self.element
          }

          #[cfg(not(all(target_arch = "wasm32", feature = "web")))]
          fn hydration_id(&self) -> usize {
            self.id
          }

          generate_html_tags! { @void $($void)? }
        }

        #[$meta]
        pub fn $tag(cx: Scope) -> HtmlElement<[<$tag:camel $($trailing_)?>]> {
          HtmlElement::new(cx, [<$tag:camel $($trailing_)?>]::default())
        }
      )*
    }
  };

  (@void) => {};
  (@void void) => {
    fn is_void(&self) -> bool {
      true
    }
  }
}

generate_html_tags![
  // ==========================
  //        Main root
  // ==========================
  /// The `<html>` HTML element represents the root (top-level element) of an HTML document, so it is also referred to as the root element. All other elements must be descendants of this element.
  html,
  // ==========================
  //     Document Metadata
  // ==========================
  /// The `<base>` HTML element specifies the base URL to use for all relative URLs in a document. There can be only one `<base>` element in a document.
  #[void]
  base,
  ///	The `<head>` HTML element contains machine-readable information (metadata) about the document, like its title, scripts, and style sheets.
  head,
  ///	The `<link>` HTML element specifies relationships between the current document and an external resource. This element is most commonly used to link to CSS, but is also used to establish site icons (both "favicon" style icons and icons for the home screen and apps on mobile devices) among other things.
  #[void]
  link,
  ///	The `<meta>` HTML element represents Metadata that cannot be represented by other HTML meta-related elements, like base, link, script, style or title.
  #[void]
  meta,
  ///	The `<style>` HTML element contains style information for a document, or part of a document. It contains CSS, which is applied to the contents of the document containing the `<style>` element.
  style,
  ///	The `<title>` HTML element defines the document's title that is shown in a Browser's title bar or a page's tab. It only contains text; tags within the element are ignored.
  title,
  // ==========================
  //     Sectioning Root
  // ==========================
  /// The `<body>` HTML element represents the content of an HTML document. There can be only one `<body>` element in a document.
  body,
  // ==========================
  //     Content Sectioning
  // ==========================
  /// The `<address>` HTML element indicates that the enclosed HTML provides contact information for a person or people, or for an organization.
  address,
  /// The `<article>` HTML element represents a self-contained composition in a document, page, application, or site, which is intended to be independently distributable or reusable (e.g., in syndication). Examples include: a forum post, a magazine or newspaper article, or a blog entry, a product card, a user-submitted comment, an interactive widget or gadget, or any other independent item of content.
  article,
  /// The `<aside>` HTML element represents a portion of a document whose content is only indirectly related to the document's main content. Asides are frequently presented as sidebars or call-out boxes.
  aside,
  /// The `<footer>` HTML element represents a footer for its nearest sectioning content or sectioning root element. A `<footer>` typically contains information about the author of the section, copyright data or links to related documents.
  footer,
  /// The `<header>` HTML element represents introductory content, typically a group of introductory or navigational aids. It may contain some heading elements but also a logo, a search form, an author name, and other elements.
  header,
  /// The `<h1>` to `<h6>` HTML elements represent six levels of section headings. `<h1>` is the highest section level and `<h6>` is the lowest.
  h1,
  /// The `<h1>` to `<h6>` HTML elements represent six levels of section headings. `<h1>` is the highest section level and `<h6>` is the lowest.
  h2,
  /// The `<h1>` to `<h6>` HTML elements represent six levels of section headings. `<h1>` is the highest section level and `<h6>` is the lowest.
  h3,
  /// The `<h1>` to `<h6>` HTML elements represent six levels of section headings. `<h1>` is the highest section level and `<h6>` is the lowest.
  h4,
  /// The `<h1>` to `<h6>` HTML elements represent six levels of section headings. `<h1>` is the highest section level and `<h6>` is the lowest.
  h5,
  /// The `<h1>` to `<h6>` HTML elements represent six levels of section headings. `<h1>` is the highest section level and `<h6>` is the lowest.
  h6,
  /// The `<main>` HTML element represents the dominant content of the body of a document. The main content area consists of content that is directly related to or expands upon the central topic of a document, or the central functionality of an application.
  main,
  /// The `<nav>` HTML element represents a section of a page whose purpose is to provide navigation links, either within the current document or to other documents. Common examples of navigation sections are menus, tables of contents, and indexes.
  nav,
  /// The `<section>` HTML element represents a generic standalone section of a document, which doesn't have a more specific semantic element to represent it. Sections should always have a heading, with very few exceptions.
  section,
  // ==========================
  //      Text Content
  // ==========================
  /// The `<blockquote>` HTML element indicates that the enclosed text is an extended quotation. Usually, this is rendered visually by indentation (see Notes for how to change it). A URL for the source of the quotation may be given using the cite attribute, while a text representation of the source can be given using the cite element.
  blockquote,
  /// The `<dd>` HTML element provides the description, definition, or value for the preceding term (dt) in a description list (dl).
  dd,
  /// The `<div>` HTML element is the generic container for flow content. It has no effect on the content or layout until styled in some way using CSS (e.g. styling is directly applied to it, or some kind of layout model like Flexbox is applied to its parent element).
  div,
  /// The `<dl>` HTML element represents a description list. The element encloses a list of groups of terms (specified using the dt element) and descriptions (provided by dd elements). Common uses for this element are to implement a glossary or to display metadata (a list of key-value pairs).
  dl,
  /// The `<dt>` HTML element specifies a term in a description or definition list, and as such must be used inside a dl element. It is usually followed by a dd element; however, multiple `<dt>` elements in a row indicate several terms that are all defined by the immediate next dd element.
  dt,
  /// The `<figcaption>` HTML element represents a caption or legend describing the rest of the contents of its parent figure element.
  figcaption,
  /// The `<figure>` HTML element represents self-contained content, potentially with an optional caption, which is specified using the figcaption element. The figure, its caption, and its contents are referenced as a single unit.
  figure,
  /// The `<hr>` HTML element represents a thematic break between paragraph-level elements: for example, a change of scene in a story, or a shift of topic within a section.
  #[void]
  hr,
  /// The `<li>` HTML element is used to represent an item in a list. It must be contained in a parent element: an ordered list (ol), an unordered list (ul), or a menu (menu). In menus and unordered lists, list items are usually displayed using bullet points. In ordered lists, they are usually displayed with an ascending counter on the left, such as a number or letter.
  li,
  /// The `<ol>` HTML element represents an ordered list of items — typically rendered as a numbered list.
  ol,
  /// The `<p>` HTML element represents a paragraph. Paragraphs are usually represented in visual media as blocks of text separated from adjacent blocks by blank lines and/or first-line indentation, but HTML paragraphs can be any structural grouping of related content, such as images or form fields.
  p,
  /// The `<pre>` HTML element represents preformatted text which is to be presented exactly as written in the HTML file. The text is typically rendered using a non-proportional, or "monospaced, font. Whitespace inside this element is displayed as written.
  pre,
  /// The `<ul>` HTML element represents an unordered list of items, typically rendered as a bulleted list.
  ul,
  // ==========================
  //    Inline Text Semantics
  // ==========================
  /// The `<a>` HTML element (or anchor element), with its href attribute, creates a hyperlink to web pages, files, email addresses, locations in the same page, or anything else a URL can address.
  a,
  /// The `<abbr>` HTML element represents an abbreviation or acronym; the optional title attribute can provide an expansion or description for the abbreviation. If present, title must contain this full description and nothing else.
  abbr,
  /// The `<b>` HTML element is used to draw the reader's attention to the element's contents, which are not otherwise granted special importance. This was formerly known as the Boldface element, and most browsers still draw the text in boldface. However, you should not use `<b>` for styling text; instead, you should use the CSS font-weight property to create boldface text, or the strong element to indicate that text is of special importance.
  b,
  /// The `<bdi>` HTML element tells the browser's bidirectional algorithm to treat the text it contains in isolation from its surrounding text. It's particularly useful when a website dynamically inserts some text and doesn't know the directionality of the text being inserted.
  bdi,
  /// The `<bdo>` HTML element overrides the current directionality of text, so that the text within is rendered in a different direction.
  bdo,
  /// The `<br>` HTML element produces a line break in text (carriage-return). It is useful for writing a poem or an address, where the division of lines is significant.
  #[void]
  br,
  /// The `<cite>` HTML element is used to describe a reference to a cited creative work, and must include the title of that work. The reference may be in an abbreviated form according to context-appropriate conventions related to citation metadata.
  cite,
  /// The `<code>` HTML element displays its contents styled in a fashion intended to indicate that the text is a short fragment of computer code. By default, the content text is displayed using the user agent default monospace font.
  code,
  /// The `<data>` HTML element links a given piece of content with a machine-readable translation. If the content is time- or date-related, the time element must be used.
  data,
  /// The `<dfn>` HTML element is used to indicate the term being defined within the context of a definition phrase or sentence. The p element, the dt/dd pairing, or the section element which is the nearest ancestor of the `<dfn>` is considered to be the definition of the term.
  dfn,
  /// The `<em>` HTML element marks text that has stress emphasis. The `<em>` element can be nested, with each level of nesting indicating a greater degree of emphasis.
  em,
  /// The `<i>` HTML element represents a range of text that is set off from the normal text for some reason, such as idiomatic text, technical terms, taxonomical designations, among others. Historically, these have been presented using italicized type, which is the original source of the `<i>` naming of this element.
  i,
  /// The `<kbd>` HTML element represents a span of inline text denoting textual user input from a keyboard, voice input, or any other text entry device. By convention, the user agent defaults to rendering the contents of a `<kbd>` element using its default monospace font, although this is not mandated by the HTML standard.
  kbd,
  /// The `<mark>` HTML element represents text which is marked or highlighted for reference or notation purposes, due to the marked passage's relevance or importance in the enclosing context.
  mark,
  /// The `<q>` HTML element indicates that the enclosed text is a short inline quotation. Most modern browsers implement this by surrounding the text in quotation marks. This element is intended for short quotations that don't require paragraph breaks; for long quotations use the blockquote element.
  q,
  /// The `<rp>` HTML element is used to provide fall-back parentheses for browsers that do not support display of ruby annotations using the ruby element. One `<rp>` element should enclose each of the opening and closing parentheses that wrap the rt element that contains the annotation's text.
  rp,
  /// The `<rt>` HTML element specifies the ruby text component of a ruby annotation, which is used to provide pronunciation, translation, or transliteration information for East Asian typography. The `<rt>` element must always be contained within a ruby element.
  rt,
  /// The `<ruby>` HTML element represents small annotations that are rendered above, below, or next to base text, usually used for showing the pronunciation of East Asian characters. It can also be used for annotating other kinds of text, but this usage is less common.
  ruby,
  /// The `<s>` HTML element renders text with a strikethrough, or a line through it. Use the `<s>` element to represent things that are no longer relevant or no longer accurate. However, `<s>` is not appropriate when indicating document edits; for that, use the del and ins elements, as appropriate.
  s,
  /// The `<samp>` HTML element is used to enclose inline text which represents sample (or quoted) output from a computer program. Its contents are typically rendered using the browser's default monospaced font (such as Courier or Lucida Console).
  samp,
  /// The `<small>` HTML element represents side-comments and small print, like copyright and legal text, independent of its styled presentation. By default, it renders text within it one font-size smaller, such as from small to x-small.
  small,
  /// The `<span>` HTML element is a generic inline container for phrasing content, which does not inherently represent anything. It can be used to group elements for styling purposes (using the class or id attributes), or because they share attribute values, such as lang. It should be used only when no other semantic element is appropriate. `<span>` is very much like a div element, but div is a block-level element whereas a `<span>` is an inline element.
  span,
  /// The `<strong>` HTML element indicates that its contents have strong importance, seriousness, or urgency. Browsers typically render the contents in bold type.
  strong,
  /// The `<sub>` HTML element specifies inline text which should be displayed as subscript for solely typographical reasons. Subscripts are typically rendered with a lowered baseline using smaller text.
  sub,
  /// The `<sup>` HTML element specifies inline text which is to be displayed as superscript for solely typographical reasons. Superscripts are usually rendered with a raised baseline using smaller text.
  sup,
  /// The `<time>` HTML element represents a specific period in time. It may include the datetime attribute to translate dates into machine-readable format, allowing for better search engine results or custom features such as reminders.
  time,
  /// The `<u>` HTML element represents a span of inline text which should be rendered in a way that indicates that it has a non-textual annotation. This is rendered by default as a simple solid underline, but may be altered using CSS.
  u,
  /// The `<var>` HTML element represents the name of a variable in a mathematical expression or a programming context. It's typically presented using an italicized version of the current typeface, although that behavior is browser-dependent.
  var,
  /// The `<wbr>` HTML element represents a word break opportunity—a position within text where the browser may optionally break a line, though its line-breaking rules would not otherwise create a break at that location.
  #[void]
  wbr,
  // ==========================
  //   Image and multimedia
  // ==========================
  /// The `<area>` HTML element defines an area inside an image map that has predefined clickable areas. An image map allows geometric areas on an image to be associated with Hyperlink.
  #[void]
  area,
  /// The `<audio>` HTML element is used to embed sound content in documents. It may contain one or more audio sources, represented using the src attribute or the source element: the browser will choose the most suitable one. It can also be the destination for streamed media, using a MediaStream.
  audio,
  /// The `<img>` HTML element embeds an image into the document.
  #[void]
  img,
  /// The `<map>` HTML element is used with area elements to define an image map (a clickable link area).
  map,
  /// The `<track>` HTML element is used as a child of the media elements, audio and video. It lets you specify timed text tracks (or time-based data), for example to automatically handle subtitles. The tracks are formatted in WebVTT format (.vtt files) — Web Video Text Tracks.
  #[void]
  track,
  /// The `<video>` HTML element embeds a media player which supports video playback into the document. You can use `<video>` for audio content as well, but the audio element may provide a more appropriate user experience.
  video,
  // ==========================
  //     Embedded Content
  // ==========================
  /// The `<embed>` HTML element embeds external content at the specified point in the document. This content is provided by an external application or other source of interactive content such as a browser plug-in.
  #[void]
  embed,
  /// The `<iframe>` HTML element represents a nested browsing context, embedding another HTML page into the current one.
  iframe,
  /// The `<object>` HTML element represents an external resource, which can be treated as an image, a nested browsing context, or a resource to be handled by a plugin.
  object,
  /// The `<param>` HTML element defines parameters for an object element.
  #[void]
  param,
  /// The `<picture>` HTML element contains zero or more source elements and one img element to offer alternative versions of an image for different display/device scenarios.
  picture,
  /// The `<portal>` HTML element enables the embedding of another HTML page into the current one for the purposes of allowing smoother navigation into new pages.
  portal,
  /// The `<source>` HTML element specifies multiple media resources for the picture, the audio element, or the video element. It is an empty element, meaning that it has no content and does not have a closing tag. It is commonly used to offer the same media content in multiple file formats in order to provide compatibility with a broad range of browsers given their differing support for image file formats and media file formats.
  #[void]
  source,
  // ==========================
  //      SVG and MathML
  // ==========================
  /// The svg element is a container that defines a new coordinate system and viewport. It is used as the outermost element of SVG documents, but it can also be used to embed an SVG fragment inside an SVG or HTML document.
  svg,
  /// The top-level element in MathML is `<math>.` Every valid MathML instance must be wrapped in `<math>` tags. In addition you must not nest a second `<math>` element in another, but you can have an arbitrary number of other child elements in it.
  math,
  // ==========================
  //         Scripting
  // ==========================
  /// Use the HTML `<canvas>` element with either the canvas scripting API or the WebGL API to draw graphics and animations.
  canvas,
  /// The `<noscript>` HTML element defines a section of HTML to be inserted if a script type on the page is unsupported or if scripting is currently turned off in the browser.
  noscript,
  /// The `<script>` HTML element is used to embed executable code or data; this is typically used to embed or refer to JavaScript code. The `<script>` element can also be used with other languages, such as WebGL's GLSL shader programming language and JSON.
  script,
  // ==========================
  //     Demarcating Edits
  // ==========================
  /// The `<del>` HTML element represents a range of text that has been deleted from a document. This can be used when rendering "track changes" or source code diff information, for example. The ins element can be used for the opposite purpose: to indicate text that has been added to the document.
  del,
  /// The `<ins>` HTML element represents a range of text that has been added to a document. You can use the del element to similarly represent a range of text that has been deleted from the document.
  ins,
  // ==========================
  //     Table Content
  // ==========================
  /// The `<caption>` HTML element specifies the caption (or title) of a table.
  caption,
  /// The `<col>` HTML element defines a column within a table and is used for defining common semantics on all common cells. It is generally found within a colgroup element.
  #[void]
  col,
  /// The `<colgroup>` HTML element defines a group of columns within a table.
  colgroup,
  /// The `<table>` HTML element represents tabular data — that is, information presented in a two-dimensional table comprised of rows and columns of cells containing data.
  table,
  /// The `<tbody>` HTML element encapsulates a set of table rows (tr elements), indicating that they comprise the body of the table (table).
  tbody,
  /// The `<td>` HTML element defines a cell of a table that contains data. It participates in the table model.
  td,
  /// The `<tfoot>` HTML element defines a set of rows summarizing the columns of the table.
  tfoot,
  /// The `<th>` HTML element defines a cell as header of a group of table cells. The exact nature of this group is defined by the scope and headers attributes.
  th,
  /// The `<thead>` HTML element defines a set of rows defining the head of the columns of the table.
  thead,
  /// The `<tr>` HTML element defines a row of cells in a table. The row's cells can then be established using a mix of td (data cell) and th (header cell) elements.
  tr,
  // ==========================
  //          Forms
  // ==========================
  /// The `<button>` HTML element represents a clickable button, used to submit forms or anywhere in a document for accessible, standard button functionality.
  button,
  /// The `<datalist>` HTML element contains a set of option elements that represent the permissible or recommended options available to choose from within other controls.
  datalist,
  /// The `<fieldset>` HTML element is used to group several controls as well as labels (label) within a web form.
  fieldset,
  /// The `<form>` HTML element represents a document section containing interactive controls for submitting information.
  form,
  /// The `<input>` HTML element is used to create interactive controls for web-based forms in order to accept data from the user; a wide variety of types of input data and control widgets are available, depending on the device and user agent. The `<input>` element is one of the most powerful and complex in all of HTML due to the sheer number of combinations of input types and attributes.
  #[void]
  input,
  /// The `<label>` HTML element represents a caption for an item in a user interface.
  label,
  /// The `<legend>` HTML element represents a caption for the content of its parent fieldset.
  legend,
  /// The `<meter>` HTML element represents either a scalar value within a known range or a fractional value.
  meter,
  /// The `<optgroup>` HTML element creates a grouping of options within a select element.
  optgroup,
  /// The `<option>` HTML element is used to define an item contained in a select, an optgroup, or a datalist element. As such, `<option>` can represent menu items in popups and other lists of items in an HTML document.
  option[_],
  /// The `<output>` HTML element is a container element into which a site or app can inject the results of a calculation or the outcome of a user action.
  output,
  /// The `<progress>` HTML element displays an indicator showing the completion progress of a task, typically displayed as a progress bar.
  progress,
  /// The `<select>` HTML element represents a control that provides a menu of options:
  select,
  /// The `<textarea>` HTML element represents a multi-line plain-text editing control, useful when you want to allow users to enter a sizeable amount of free-form text, for example a comment on a review or feedback form.
  textarea,
  // ==========================
  //    Interactive elements
  // ==========================
  /// The `<details>` HTML element creates a disclosure widget in which information is visible only when the widget is toggled into an "open" state. A summary or label must be provided using the summary element.
  details,
  /// The `<dialog>` HTML element represents a dialog box or other interactive component, such as a dismissible alert, inspector, or subwindow.
  dialog,
  /// The `<menu>` HTML element is a semantic alternative to ul. It represents an unordered list of items (represented by li elements), each of these represent a link or other command that the user can activate.
  menu,
  /// The `<summary>` HTML element specifies a summary, caption, or legend for a details element's disclosure box. Clicking the `<summary>` element toggles the state of the parent `<details>` element open and closed.
  summary,
  // ==========================
  //      Web Components
  // ==========================
  /// The `<slot>` HTML element—part of the Web Components technology suite—is a placeholder inside a web component that you can fill with your own markup, which lets you create separate DOM trees and present them together.
  slot,
  /// The `<template>` HTML element is a mechanism for holding HTML that is not to be rendered immediately when a page is loaded but may be instantiated subsequently during runtime using JavaScript.
  template,
];
