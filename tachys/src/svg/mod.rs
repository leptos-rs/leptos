use crate::{
    html::{
        attribute::{any_attribute::AnyAttribute, Attribute},
        element::{ElementType, ElementWithChildren, HtmlElement},
    },
    hydration::Cursor,
    prelude::{AddAnyAttr, Mountable},
    renderer::{
        dom::{Element, Node},
        CastFrom, Rndr,
    },
    view::{Position, PositionState, Render, RenderHtml},
};
use std::{borrow::Cow, fmt::Debug};

macro_rules! svg_elements {
	($($tag:ident  [$($attr:ty),*]),* $(,)?) => {
        paste::paste! {
            $(
                /// An SVG element.
                // `tag()` function
                #[allow(non_snake_case)]
                #[track_caller]
                pub fn $tag() -> HtmlElement<[<$tag:camel>], (), ()>
                where
                {
                    HtmlElement {
                        #[cfg(any(debug_assertions, leptos_debuginfo))]
                        defined_at: std::panic::Location::caller(),
                        tag: [<$tag:camel>],
                        attributes: (),
                        children: (),
                    }
                }

                /// An SVG element.
                #[derive(Debug, Copy, Clone, PartialEq, Eq)]
                pub struct [<$tag:camel>];

				impl<At, Ch> HtmlElement<[<$tag:camel>], At, Ch>
				where
					At: Attribute,
					Ch: Render,

				{
					$(
                        pub fn $attr<V>(self, value: V) -> HtmlElement <
                            [<$tag:camel>],
                            <At as $crate::html::attribute::NextAttribute<Attr<$crate::html::attribute::[<$attr:camel>], V>>>::Output,
                            Ch
                        >
                        where
                            V: AttributeValue,
                            At: $crate::html::attribute::NextAttribute<Attr<$crate::html::attribute::[<$attr:camel>], V>>,
                            <At as $crate::html::attribute::NextAttribute<Attr<$crate::html::attribute::[<$attr:camel>], V>>>::Output: Attribute,
                        {
                            let HtmlElement { tag, children, attributes,
                                #[cfg(any(debug_assertions, leptos_debuginfo))]
                                defined_at
                            } = self;
                            HtmlElement {
                                tag,

                                children,
                                attributes: attributes.add_any_attr($crate::html::attribute::$attr(value)),
                                #[cfg(any(debug_assertions, leptos_debuginfo))]
                                defined_at
                            }
                        }
					)*
				}

                impl ElementType for [<$tag:camel>] {
                    type Output = web_sys::SvgElement;

                    const TAG: &'static str = stringify!($tag);
                    const SELF_CLOSING: bool = false;
                    const ESCAPE_CHILDREN: bool = true;
                    const NAMESPACE: Option<&'static str> = Some("http://www.w3.org/2000/svg");

                    #[inline(always)]
                    fn tag(&self) -> &str {
                        Self::TAG
                    }
                }

                impl ElementWithChildren for [<$tag:camel>] {}
            )*
		}
    }
}

svg_elements![
  a [],
  animate [],
  animateMotion [],
  animateTransform [],
  circle [],
  clipPath [],
  defs [],
  desc [],
  discard [],
  ellipse [],
  feBlend [],
  feColorMatrix [],
  feComponentTransfer [],
  feComposite [],
  feConvolveMatrix [],
  feDiffuseLighting [],
  feDisplacementMap [],
  feDistantLight [],
  feDropShadow [],
  feFlood [],
  feFuncA [],
  feFuncB [],
  feFuncG [],
  feFuncR [],
  feGaussianBlur [],
  feImage [],
  feMerge [],
  feMergeNode [],
  feMorphology [],
  feOffset [],
  fePointLight [],
  feSpecularLighting [],
  feSpotLight [],
  feTile [],
  feTurbulence [],
  filter [],
  foreignObject [],
  g [],
  hatch [],
  hatchpath [],
  image [],
  line [],
  linearGradient [],
  marker [],
  mask [],
  metadata [],
  mpath [],
  path [],
  pattern [],
  polygon [],
  polyline [],
  radialGradient [],
  rect [],
  script [],
  set [],
  stop [],
  style [],
  svg [],
  switch [],
  symbol [],
  text [],
  textPath [],
  title [],
  tspan [],
  view [],
];

/// An SVG element.
#[allow(non_snake_case)]
#[track_caller]
pub fn r#use() -> HtmlElement<Use, (), ()>
where {
    HtmlElement {
        #[cfg(any(debug_assertions, leptos_debuginfo))]
        defined_at: std::panic::Location::caller(),
        tag: Use,
        attributes: (),
        children: (),
    }
}

/// An SVG element.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Use;

impl ElementType for Use {
    type Output = web_sys::SvgElement;

    const TAG: &'static str = "use";
    const SELF_CLOSING: bool = false;
    const ESCAPE_CHILDREN: bool = true;
    const NAMESPACE: Option<&'static str> = Some("http://www.w3.org/2000/svg");

    #[inline(always)]
    fn tag(&self) -> &str {
        Self::TAG
    }
}

impl ElementWithChildren for Use {}

/// An element that contains no interactivity, and whose contents can be known at compile time.
pub struct InertElement {
    html: Cow<'static, str>,
}

impl InertElement {
    /// Creates a new inert svg element.
    pub fn new(html: impl Into<Cow<'static, str>>) -> Self {
        Self { html: html.into() }
    }
}

/// Retained view state for [`InertElement`].
pub struct InertElementState(Cow<'static, str>, Element);

impl Mountable for InertElementState {
    fn unmount(&mut self) {
        self.1.unmount();
    }

    fn mount(&mut self, parent: &Element, marker: Option<&Node>) {
        self.1.mount(parent, marker)
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        self.1.insert_before_this(child)
    }

    fn elements(&self) -> Vec<crate::renderer::types::Element> {
        vec![self.1.clone()]
    }
}

impl Render for InertElement {
    type State = InertElementState;

    fn build(self) -> Self::State {
        let el = Rndr::create_svg_element_from_html(self.html.clone());
        InertElementState(self.html, el)
    }

    fn rebuild(self, state: &mut Self::State) {
        let InertElementState(prev, el) = state;
        if &self.html != prev {
            let mut new_el =
                Rndr::create_svg_element_from_html(self.html.clone());
            el.insert_before_this(&mut new_el);
            el.unmount();
            *el = new_el;
            *prev = self.html;
        }
    }
}

impl AddAnyAttr for InertElement {
    type Output<SomeNewAttr: Attribute> = Self;

    fn add_any_attr<NewAttr: Attribute>(
        self,
        _attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        panic!(
            "InertElement does not support adding attributes. It should only \
             be used as a child, and not returned at the top level."
        )
    }
}

impl RenderHtml for InertElement {
    type AsyncOutput = Self;
    type Owned = Self;

    const MIN_LENGTH: usize = 0;

    fn html_len(&self) -> usize {
        self.html.len()
    }

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self {
        self
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        _escape: bool,
        _mark_branches: bool,
        _extra_attrs: Vec<AnyAttribute>,
    ) {
        buf.push_str(&self.html);
        *position = Position::NextChild;
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor,
        position: &PositionState,
    ) -> Self::State {
        let curr_position = position.get();
        if curr_position == Position::FirstChild {
            cursor.child();
        } else if curr_position != Position::Current {
            cursor.sibling();
        }
        let el = crate::renderer::types::Element::cast_from(cursor.current())
            .unwrap();
        position.set(Position::NextChild);
        InertElementState(self.html, el)
    }

    fn into_owned(self) -> Self::Owned {
        self
    }
}
