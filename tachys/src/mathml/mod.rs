use crate::{
    html::{
        attribute::{Attr, Attribute, AttributeValue},
        element::{ElementType, ElementWithChildren, HtmlElement},
    },
    view::Render,
};
use next_tuple::NextTuple;
use std::fmt::Debug;

macro_rules! mathml_global {
	($tag:ty, $attr:ty) => {
		paste::paste! {
            /// A MathML attribute.
			pub fn $attr<V>(self, value: V) -> HtmlElement <
				[<$tag:camel>],
				<At as NextTuple>::Output<Attr<$crate::html::attribute::[<$attr:camel>], V>>,
				Ch
			>
			where
				V: AttributeValue,
				At: NextTuple,
				<At as NextTuple>::Output<Attr<$crate::html::attribute::[<$attr:camel>], V>>: Attribute,
			{
				let HtmlElement { tag, children, attributes } = self;
				HtmlElement {
					tag,

					children,
					attributes: attributes.next_tuple($crate::html::attribute::$attr(value)),
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
                /// A MathML element.
                #[track_caller]
                pub fn $tag() -> HtmlElement<[<$tag:camel>], (), ()>
                where

                {
                    HtmlElement {
                        tag: [<$tag:camel>],
                        attributes: (),
                        children: (),

                    }
                }

                /// A MathML element.
                #[derive(Debug, Copy, Clone, PartialEq, Eq)]
                pub struct [<$tag:camel>];

				impl<At, Ch> HtmlElement<[<$tag:camel>], At, Ch>
				where
					At: Attribute,
					Ch: Render,

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
                        /// A MathML attribute.
                        pub fn $attr<V>(self, value: V) -> HtmlElement <
                            [<$tag:camel>],
                            <At as NextTuple>::Output<Attr<$crate::html::attribute::[<$attr:camel>], V>>,
                            Ch
                        >
                        where
                            V: AttributeValue,
                            At: NextTuple,
                            <At as NextTuple>::Output<Attr<$crate::html::attribute::[<$attr:camel>], V>>: Attribute,
                        {
                            let HtmlElement { tag, children, attributes } = self;
                            HtmlElement {
                                tag,

                                children,
                                attributes: attributes.next_tuple($crate::html::attribute::$attr(value)),
                            }
                        }
					)*
				}

                impl ElementType for [<$tag:camel>] {
                    type Output = web_sys::Element;

                    const TAG: &'static str = stringify!($tag);
                    const SELF_CLOSING: bool = false;
                    const ESCAPE_CHILDREN: bool = true;
                    const NAMESPACE: Option<&'static str> = Some("http://www.w3.org/1998/Math/MathML");

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

mathml_elements![
    math [display, xmlns],
    mi [],
    mn [],
    mo [
        accent, fence, lspace, maxsize, minsize, movablelimits,
        rspace, separator, stretchy, symmetric, form
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
