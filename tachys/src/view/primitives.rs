use super::{Mountable, Position, PositionState, Render, RenderHtml};
use crate::{
    html::attribute::any_attribute::AnyAttribute,
    hydration::Cursor,
    no_attrs,
    renderer::{CastFrom, Rndr},
    view::ToTemplate,
};
use std::{
    fmt::Write,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    num::{
        NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8,
        NonZeroIsize, NonZeroU128, NonZeroU16, NonZeroU32, NonZeroU64,
        NonZeroU8, NonZeroUsize,
    },
};

// any changes here should also be made in src/reactive_graph/guards.rs
macro_rules! render_primitive {
  ($escape:literal; $($child_type:ty),* $(,)?) => {
    $(
		paste::paste! {
			pub struct [<$child_type:camel State>](crate::renderer::types::Text, $child_type);

			impl Mountable for [<$child_type:camel State>] {
					fn unmount(&mut self) {
						self.0.unmount()
					}

					fn mount(
						&mut self,
						parent: &crate::renderer::types::Element,
						marker: Option<&crate::renderer::types::Node>,
					) {
						Rndr::insert_node(parent, self.0.as_ref(), marker);
					}

					fn insert_before_this(&self,
						child: &mut dyn Mountable,
					) -> bool {
                        self.0.insert_before_this(child)
					}

					fn elements(&self) -> Vec<crate::renderer::types::Element> {
						vec![]
					}
			}

			impl Render for $child_type {
				type State = [<$child_type:camel State>];


				fn build(self) -> Self::State {
					let node = Rndr::create_text_node(&self.to_string());
					[<$child_type:camel State>](node, self)
				}

				fn rebuild(self, state: &mut Self::State) {
					let [<$child_type:camel State>](node, this) = state;
					if &self != this {
						Rndr::set_text(node, &self.to_string());
						*this = self;
					}
				}
			}

            no_attrs!($child_type);

			impl RenderHtml for $child_type
			{
				type AsyncOutput = Self;
				type Owned = Self;

				const MIN_LENGTH: usize = 0;

                fn dry_resolve(&mut self) {}

                async fn resolve(self) -> Self::AsyncOutput {
                    self
                }

				fn to_html_with_buf(self, buf: &mut String, position: &mut Position, escape: bool, _mark_branches: bool, _extra_attrs: Vec<AnyAttribute>) {
					// add a comment node to separate from previous sibling, if any
					if matches!(position, Position::NextChildAfterText) {
						buf.push_str("<!>")
					}
					// `$escape` is `true` only for types whose `Display` output can
					// contain HTML-significant characters (e.g. `char`). Numeric
					// primitives emit a syntactic subset of HTML text and are written
					// directly to avoid an intermediate allocation.
					if $escape {
						if escape {
							buf.push_str(&html_escape::encode_text(&self.to_string()));
						} else {
							_ = write!(buf, "{}", self);
						}
					} else {
						_ = write!(buf, "{}", self);
					}
					*position = Position::NextChildAfterText;
				}

				fn hydrate<const FROM_SERVER: bool>(
					self,
					cursor: &Cursor,
					position: &PositionState,
				) -> Self::State {
					if position.get() == Position::FirstChild {
						cursor.child();
					} else {
						cursor.sibling();
					}

					// separating placeholder marker comes before text node
					if matches!(position.get(), Position::NextChildAfterText) {
						cursor.sibling();
					}

					let node = cursor.current();
					let node = crate::renderer::types::Text::cast_from(node.clone())
						.unwrap_or_else(|| crate::hydration::failed_to_cast_text_node(node));

					if !FROM_SERVER {
						Rndr::set_text(&node, &self.to_string());
					}
					position.set(Position::NextChildAfterText);

					[<$child_type:camel State>](node, self)
				}

				fn into_owned(self) -> Self::Owned {
					self
				}
			}

			impl<'a> ToTemplate for $child_type {
				const TEMPLATE: &'static str = " <!>";

				fn to_template(
					buf: &mut String,
					_class: &mut String,
					_style: &mut String,
					_inner_html: &mut String,
					position: &mut Position,
				) {
					if matches!(*position, Position::NextChildAfterText) {
						buf.push_str("<!>")
					}
					buf.push(' ');
					*position = Position::NextChildAfterText;
				}
			}
		}
    )*
  };
}

render_primitive![
    false;
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
    bool,
    IpAddr,
    SocketAddr,
    SocketAddrV4,
    SocketAddrV6,
    Ipv4Addr,
    Ipv6Addr,
    NonZeroI8,
    NonZeroU8,
    NonZeroI16,
    NonZeroU16,
    NonZeroI32,
    NonZeroU32,
    NonZeroI64,
    NonZeroU64,
    NonZeroI128,
    NonZeroU128,
    NonZeroIsize,
    NonZeroUsize,
];

// `char` can be any Unicode scalar value, including HTML-significant
// characters such as `<`, `>`, `&`, `"`, and `'`. Its body output must be
// escaped to avoid an HTML-injection sink.
render_primitive![true; char];

#[cfg(test)]
mod tests {
    use crate::view::{Position, RenderHtml};

    #[test]
    fn char_escapes_html_special_characters() {
        let mut buf = String::new();
        '<'.to_html_with_buf(
            &mut buf,
            &mut Position::FirstChild,
            true,
            false,
            vec![],
        );
        assert_eq!(buf, "&lt;");
    }

    #[test]
    fn char_not_escaped_in_raw_text_context() {
        // When the enclosing element opts out of escaping (e.g. `<script>`),
        // the char must be written verbatim.
        let mut buf = String::new();
        '<'.to_html_with_buf(
            &mut buf,
            &mut Position::FirstChild,
            false,
            false,
            vec![],
        );
        assert_eq!(buf, "<");
    }

    #[test]
    fn numeric_primitive_renders_unescaped() {
        let mut buf = String::new();
        42u32.to_html_with_buf(
            &mut buf,
            &mut Position::FirstChild,
            true,
            false,
            vec![],
        );
        assert_eq!(buf, "42");
    }
}
