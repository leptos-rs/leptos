//! Implements the [`Render`] and [`RenderHtml`] traits for signal guard types.

use crate::{
    html::attribute::Attribute,
    hydration::Cursor,
    prelude::RenderHtml,
    renderer::{CastFrom, Renderer},
    view::{
        add_attr::AddAnyAttr, strings::StrState, Mountable, Position,
        PositionState, Render, ToTemplate,
    },
};
use reactive_graph::signal::guards::ReadGuard;
use std::{
    fmt::Write,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    num::{
        NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8,
        NonZeroIsize, NonZeroU128, NonZeroU16, NonZeroU32, NonZeroU64,
        NonZeroU8, NonZeroUsize,
    },
    ops::Deref,
};

// any changes here should also be made in src/view/primitives.rs
// TODO should also apply to mapped signal read guards
macro_rules! render_primitive {
  ($($child_type:ty),* $(,)?) => {
    $(
		paste::paste! {
			pub struct [<ReadGuard $child_type:camel State>]<R>(R::Text, $child_type) where R: Renderer;

			impl<'a, R: Renderer> Mountable<R> for [<ReadGuard $child_type:camel State>]<R> {
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

			impl<G, R: Renderer> Render<R> for ReadGuard<$child_type, G>
            where G: Deref<Target = $child_type>
            {
				type State = [<ReadGuard $child_type:camel State>]<R>;


				fn build(self) -> Self::State {
					let node = R::create_text_node(&self.to_string());
					[<ReadGuard $child_type:camel State>](node, *self)
				}

				fn rebuild(self, state: &mut Self::State) {
					let [<ReadGuard $child_type:camel State>](node, this) = state;
					if &self != this {
						R::set_text(node, &self.to_string());
						*this = *self;
					}
				}
			}

            impl<G, R> AddAnyAttr<R> for ReadGuard<$child_type, G>
			where
				R: Renderer,
                G: Deref<Target = $child_type> + Send
            {
                type Output<SomeNewAttr: Attribute<R>> = ReadGuard<$child_type, G>;

                fn add_any_attr<NewAttr: Attribute<R>>(
                    self,
                    attr: NewAttr,
                ) -> Self::Output<NewAttr>
                where
                    Self::Output<NewAttr>: RenderHtml<R>,
                {
                    // TODO: there is a strange compiler thing that seems to prevent us returning Self here,
                    // even though we've already said that Output is always the same as Self
                    todo!()
                }
            }

			impl<G, R> RenderHtml<R> for ReadGuard<$child_type, G>
			where
				R: Renderer,
                G: Deref<Target = $child_type> + Send
			{
                type AsyncOutput = Self;

				const MIN_LENGTH: usize = 0;

				fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
					// add a comment node to separate from previous sibling, if any
					if matches!(position, Position::NextChildAfterText) {
						buf.push_str("<!>")
					}
					_ = write!(buf, "{}", self);
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

					[<ReadGuard $child_type:camel State>](node, *self)
				}

                async fn resolve(self) -> Self::AsyncOutput {
                    self
                }
			}

		    impl<'a, G> ToTemplate for ReadGuard<$child_type, G>
            {
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

// strings
pub struct ReadGuardStringState<R: Renderer> {
    node: R::Text,
    str: String,
}

impl<G, R: Renderer> Render<R> for ReadGuard<String, G>
where
    G: Deref<Target = String>,
{
    type State = ReadGuardStringState<R>;

    fn build(self) -> Self::State {
        let node = R::create_text_node(&self);
        ReadGuardStringState {
            node,
            str: self.to_string(),
        }
    }

    fn rebuild(self, state: &mut Self::State) {
        let ReadGuardStringState { node, str } = state;
        if *self != *str {
            R::set_text(node, &self);
            str.clear();
            str.push_str(&self);
        }
    }
}

impl<G, R> AddAnyAttr<R> for ReadGuard<String, G>
where
    G: Deref<Target = String> + Send,
    R: Renderer,
{
    type Output<SomeNewAttr: Attribute<R>> = ReadGuard<String, G>;

    fn add_any_attr<NewAttr: Attribute<R>>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<R>,
    {
        // TODO: there is a strange compiler thing that seems to prevent us returning Self here,
        // even though we've already said that Output is always the same as Self
        todo!()
    }
}

impl<G, R> RenderHtml<R> for ReadGuard<String, G>
where
    R: Renderer,
    G: Deref<Target = String> + Send,
{
    type AsyncOutput = Self;

    const MIN_LENGTH: usize = 0;

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        <&str as RenderHtml<R>>::to_html_with_buf(&self, buf, position)
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State {
        let this: &str = self.as_ref();
        let StrState { node, .. } =
            this.hydrate::<FROM_SERVER>(cursor, position);
        ReadGuardStringState {
            node,
            str: self.to_string(),
        }
    }

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }
}

impl<G> ToTemplate for ReadGuard<String, G> {
    const TEMPLATE: &'static str = <&str as ToTemplate>::TEMPLATE;

    fn to_template(
        buf: &mut String,
        class: &mut String,
        style: &mut String,
        inner_html: &mut String,
        position: &mut Position,
    ) {
        <&str as ToTemplate>::to_template(
            buf, class, style, inner_html, position,
        )
    }
}

impl<R: Renderer> Mountable<R> for ReadGuardStringState<R> {
    fn unmount(&mut self) {
        self.node.unmount()
    }

    fn mount(
        &mut self,
        parent: &<R as Renderer>::Element,
        marker: Option<&<R as Renderer>::Node>,
    ) {
        R::insert_node(parent, self.node.as_ref(), marker);
    }

    fn insert_before_this(
        &self,
        parent: &<R as Renderer>::Element,
        child: &mut dyn Mountable<R>,
    ) -> bool {
        child.mount(parent, Some(self.node.as_ref()));
        true
    }
}
