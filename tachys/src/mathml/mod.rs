use crate::{
    html::{
        attribute::{Attr, Attribute, AttributeValue},
        element::{
            CreateElement, ElementType, ElementWithChildren, HtmlElement,
        },
    },
    renderer::{dom::Dom, Renderer},
    view::Render,
};
use next_tuple::TupleBuilder;
use once_cell::unsync::Lazy;
use std::{fmt::Debug, marker::PhantomData};

macro_rules! mathml_global {
	($tag:ty, $attr:ty) => {
		paste::paste! {
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
		}
	}
}

macro_rules! mathml_elements {
	($($tag:ident  [$($attr:ty),*]),* $(,)?) => {
        paste::paste! {
            $(
                // `tag()` function
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
					mathml_global!($tag, displaystyle);
					mathml_global!($tag, href);
					mathml_global!($tag, id);
					mathml_global!($tag, mathbackground);
					mathml_global!($tag, mathcolor);
					mathml_global!($tag, mathsize);
					mathml_global!($tag, mathvariant);
					mathml_global!($tag, scriptlevel);

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
                    type Output = web_sys::Element;

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
									Some(wasm_bindgen::intern("http://www.w3.org/1998/Math/MathML")),
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

mathml_elements![
    math [display, xmlns],
    mi [],
    mn [],
    mo [
        accent, fence, lspace, maxsize, minsize, movablelimits,
        rspace, separator, stretchy, symmetric
    ],
    ms [],
    mspace [height, width],
    mtext [],
    menclose [notation],
    merror [],
    mfenced [],
    mfrac [linethickness],
    mpadded [depth, height, voffset, width],
    mphantom [],
    mroot [],
    mrow [],
    msqrt [],
    mstyle [],
    mmultiscripts [],
    mover [accent],
    mprescripts [],
    msub [],
    msubsup [],
    msup [],
    munder [accentunder],
    munderover [accent, accentunder],
    mtable [
        align, columnalign, columnlines, columnspacing, frame,
        framespacing, rowalign, rowlines, rowspacing, width
    ],
    mtd [columnalign, columnspan, rowalign, rowspan],
    mtr [columnalign, rowalign],
    maction [],
    annotation [],
    semantics [],
];
