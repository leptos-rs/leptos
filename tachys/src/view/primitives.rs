use super::{Mountable, Position, PositionState, Render, RenderHtml};
use crate::{
    hydration::Cursor,
    renderer::{CastFrom, Renderer},
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
  ($($child_type:ty),* $(,)?) => {
    $(
		paste::paste! {
			pub struct [<$child_type:camel State>]<R>(R::Text, $child_type) where R: Renderer;

			impl<R: Renderer> Mountable<R> for [<$child_type:camel State>]<R> {
					fn unmount(&mut self) {
						self.0.unmount()
					}

					fn mount(
						&mut self,
						parent: &<R as Renderer>::Element,
						marker: Option<&<R as Renderer>::Node>,
					) {
						R::insert_node(parent, self.0.as_ref(), marker);
					}

					fn insert_before_this(
						&self,
						parent: &<R as Renderer>::Element,
						child: &mut dyn Mountable<R>,
					) -> bool {
						child.mount(parent, Some(self.0.as_ref()));
						true
					}
			}

			impl<R: Renderer> Render<R> for $child_type {
				type State = [<$child_type:camel State>]<R>;
                type FallibleState = Self::State;
				type AsyncOutput = Self;

				fn build(self) -> Self::State {
					let node = R::create_text_node(&self.to_string());
					[<$child_type:camel State>](node, self)
				}

				fn rebuild(self, state: &mut Self::State) {
					let [<$child_type:camel State>](node, this) = state;
					if &self != this {
						R::set_text(node, &self.to_string());
						*this = self;
					}
				}

                fn try_build(self) -> any_error::Result<Self::FallibleState> {
                    Ok(self.build())
                }

                fn try_rebuild(self, state: &mut Self::FallibleState) -> any_error::Result<()> {
                    self.rebuild(state);
					Ok(())
                }

                async fn resolve(self) -> Self::AsyncOutput {
                    self
                }
			}

			impl<R> RenderHtml<R> for $child_type
			where
				R: Renderer,


			{
				const MIN_LENGTH: usize = 0;

				fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
					// add a comment node to separate from previous sibling, if any
					if matches!(position, Position::NextChildAfterText) {
						buf.push_str("<!>")
					}
					if let Err(e) = write!(buf, "{}", self) {
                        #[cfg(feature = "tracing")]
                        tracing::error!(e);
                        #[cfg(not(feature = "tracing"))]
                        { _ = e;}
                    }
					*position = Position::NextChildAfterText;
				}

				fn hydrate<const FROM_SERVER: bool>(
					self,
					cursor: &Cursor<R>,
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
					let node = R::Text::cast_from(node)
						.expect("couldn't cast text node from node");

					if !FROM_SERVER {
						R::set_text(&node, &self.to_string());
					}
					position.set(Position::NextChildAfterText);

					[<$child_type:camel State>](node, self)
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
