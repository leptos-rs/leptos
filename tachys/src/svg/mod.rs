use crate::{
    html::{
        attribute::Attribute,
        element::{ElementType, ElementWithChildren, HtmlElement},
    },
    view::Render,
};
use std::fmt::Debug;

macro_rules! svg_elements {
	($($tag:ident  [$($attr:ty),*]),* $(,)?) => {
        paste::paste! {
            $(
                /// An SVG element.
                // `tag()` function
                #[allow(non_snake_case)]
                pub fn $tag() -> HtmlElement<[<$tag:camel>], (), ()>
                where

                {
                    HtmlElement {
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
                            <At as NextTuple<Attr<$crate::html::attribute::[<$attr:camel>], V>>>::Output,
                            Ch
                        >
                        where
                            V: AttributeValue,
                            At: NextTuple<Attr<$crate::html::attribute::[<$attr:camel>], V>>,
                            <At as NextTuple<Attr<$crate::html::attribute::[<$attr:camel>], V>>>::Output: Attribute,
                        {
                            let HtmlElement { tag, children, attributes,
                                #[cfg(debug_assertions)]
                                defined_at
                            } = self;
                            HtmlElement {
                                tag,

                                children,
                                attributes: attributes.next_tuple($crate::html::attribute::$attr(value)),
                                #[cfg(debug_assertions)]
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
pub fn r#use() -> HtmlElement<Use, (), ()>
where {
    HtmlElement {
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
