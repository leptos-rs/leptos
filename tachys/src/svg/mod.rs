use crate::{
    html::{
        attribute::Attribute,
        element::{
            CreateElement, ElementType, ElementWithChildren, HtmlElement,
        },
    },
    renderer::{dom::Dom, Renderer},
    view::Render,
};
use once_cell::unsync::Lazy;
use std::{fmt::Debug, marker::PhantomData};

macro_rules! svg_elements {
	($($tag:ident  [$($attr:ty),*]),* $(,)?) => {
        paste::paste! {
            $(
                // `tag()` function
                #[allow(non_snake_case)]
                pub fn $tag<Rndr>() -> HtmlElement<[<$tag:camel>], (), (), Rndr>
                where
                    Rndr: Renderer
                {
                    HtmlElement {
                        tag: [<$tag:camel>],
                        attributes: (),
                        children: (),
                        rndr: PhantomData,
                    }
                }

                #[derive(Debug, Copy, Clone, PartialEq, Eq)]
                pub struct [<$tag:camel>];

				impl<At, Ch, Rndr> HtmlElement<[<$tag:camel>], At, Ch, Rndr>
				where
					At: Attribute<Rndr>,
					Ch: Render<Rndr>,
					Rndr: Renderer,
				{
					$(
                        pub fn $attr<V>(self, value: V) -> HtmlElement <
                            [<$tag:camel>],
                            <At as TupleBuilder<Attr<$crate::html::attribute::[<$attr:camel>], V, Rndr>>>::Output,
                            Ch, Rndr
                        >
                        where
                            V: AttributeValue<Rndr>,
                            At: TupleBuilder<Attr<$crate::html::attribute::[<$attr:camel>], V, Rndr>>,
                            <At as TupleBuilder<Attr<$crate::html::attribute::[<$attr:camel>], V, Rndr>>>::Output: Attribute<Rndr>,
                        {
                            let HtmlElement { tag, rndr, children, attributes } = self;
                            HtmlElement {
                                tag,
                                rndr,
                                children,
                                attributes: attributes.next_tuple($crate::html::attribute::$attr(value))
                            }
                        }
					)*
				}

                impl ElementType for [<$tag:camel>] {
                    type Output = web_sys::SvgElement;

                    const TAG: &'static str = stringify!($tag);
                    const SELF_CLOSING: bool = false;

                    #[inline(always)]
                    fn tag(&self) -> &str {
                        Self::TAG
                    }
                }

                impl ElementWithChildren for [<$tag:camel>] {}

                impl CreateElement<Dom> for [<$tag:camel>] {
                    fn create_element(&self) -> <Dom as Renderer>::Element {
                        use wasm_bindgen::JsCast;

                        thread_local! {
                            static ELEMENT: Lazy<<Dom as Renderer>::Element> = Lazy::new(|| {
                                crate::dom::document().create_element_ns(
									Some(wasm_bindgen::intern("http://www.w3.org/2000/svg")),
									stringify!($tag)
								).unwrap()
                            });
                        }
                        ELEMENT.with(|e| e.clone_node()).unwrap().unchecked_into()
                    }
                }
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

// TODO <use>
