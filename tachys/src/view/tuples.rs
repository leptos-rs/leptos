use super::{
    Mountable, Position, PositionState, Render, RenderHtml, ToTemplate,
};
use crate::{
    html::attribute::Attribute,
    hydration::Cursor,
    renderer::Rndr,
    view::{add_attr::AddAnyAttr, StreamBuilder},
};
use const_str_slice_concat::{
    const_concat, const_concat_with_separator, str_from_buffer,
};

impl Render for () {
    type State = crate::renderer::types::Placeholder;

    fn build(self) -> Self::State {
        Rndr::create_placeholder()
    }

    fn rebuild(self, _state: &mut Self::State) {}
}

impl RenderHtml for () {
    type AsyncOutput = ();

    const MIN_LENGTH: usize = 3;
    const EXISTS: bool = false;

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        _mark_branches: bool,
    ) {
        if escape {
            buf.push_str("<!>");
            *position = Position::NextChild;
        }
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor,
        position: &PositionState,
    ) -> Self::State {
        let marker = cursor.next_placeholder(position);
        position.set(Position::NextChild);
        marker
    }

    async fn resolve(self) -> Self::AsyncOutput {}

    fn dry_resolve(&mut self) {}
}

impl AddAnyAttr for () {
    type Output<SomeNewAttr: Attribute> = ();

    fn add_any_attr<NewAttr: Attribute>(
        self,
        _attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
    }
}

impl Mountable for () {
    fn unmount(&mut self) {}

    fn mount(
        &mut self,
        _parent: &crate::renderer::types::Element,
        _marker: Option<&crate::renderer::types::Node>,
    ) {
    }

    fn insert_before_this(&self, _child: &mut dyn Mountable) -> bool {
        false
    }
}

impl ToTemplate for () {
    const TEMPLATE: &'static str = "<!>";

    fn to_template(
        buf: &mut String,
        _class: &mut String,
        _style: &mut String,
        _inner_html: &mut String,
        _position: &mut Position,
    ) {
        buf.push_str("<!>");
    }
}

impl<A: Render> Render for (A,) {
    type State = A::State;

    fn build(self) -> Self::State {
        self.0.build()
    }

    fn rebuild(self, state: &mut Self::State) {
        self.0.rebuild(state)
    }
}

impl<A> RenderHtml for (A,)
where
    A: RenderHtml,
{
    type AsyncOutput = (A::AsyncOutput,);

    const MIN_LENGTH: usize = A::MIN_LENGTH;

    fn html_len(&self) -> usize {
        self.0.html_len()
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
    ) {
        self.0
            .to_html_with_buf(buf, position, escape, mark_branches);
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
    ) where
        Self: Sized,
    {
        self.0.to_html_async_with_buf::<OUT_OF_ORDER>(
            buf,
            position,
            escape,
            mark_branches,
        );
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor,
        position: &PositionState,
    ) -> Self::State {
        self.0.hydrate::<FROM_SERVER>(cursor, position)
    }

    async fn resolve(self) -> Self::AsyncOutput {
        (self.0.resolve().await,)
    }

    fn dry_resolve(&mut self) {
        self.0.dry_resolve();
    }
}

impl<A: ToTemplate> ToTemplate for (A,) {
    const TEMPLATE: &'static str = A::TEMPLATE;
    const CLASS: &'static str = A::CLASS;
    const STYLE: &'static str = A::STYLE;

    fn to_template(
        buf: &mut String,
        class: &mut String,
        style: &mut String,
        inner_html: &mut String,
        position: &mut Position,
    ) {
        A::to_template(buf, class, style, inner_html, position)
    }
}

impl<A> AddAnyAttr for (A,)
where
    A: AddAnyAttr,
{
    type Output<SomeNewAttr: Attribute> = (A::Output<SomeNewAttr>,);

    fn add_any_attr<NewAttr: Attribute>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        (self.0.add_any_attr(attr),)
    }
}

macro_rules! impl_view_for_tuples {
	($first:ident, $($ty:ident),* $(,)?) => {
		impl<$first, $($ty),*> Render for ($first, $($ty,)*)
		where
			$first: Render,
			$($ty: Render),*,

		{
			type State = ($first::State, $($ty::State,)*);


			fn build(self) -> Self::State {
                #[allow(non_snake_case)]
                let ($first, $($ty,)*) = self;
                (
                    $first.build(),
                    $($ty.build()),*
                )
			}

			fn rebuild(self, state: &mut Self::State) {
				paste::paste! {
					let ([<$first:lower>], $([<$ty:lower>],)*) = self;
					let ([<view_ $first:lower>], $([<view_ $ty:lower>],)*) = state;
					[<$first:lower>].rebuild([<view_ $first:lower>]);
					$([<$ty:lower>].rebuild([<view_ $ty:lower>]));*
				}
			}
		}

		impl<$first, $($ty),*> RenderHtml for ($first, $($ty,)*)
		where
			$first: RenderHtml,
			$($ty: RenderHtml),*,

		{
            type AsyncOutput = ($first::AsyncOutput, $($ty::AsyncOutput,)*);

            const MIN_LENGTH: usize = $first::MIN_LENGTH $(+ $ty::MIN_LENGTH)*;

            #[inline(always)]
            fn html_len(&self) -> usize {
                #[allow(non_snake_case)]
			    let ($first, $($ty,)* ) = self;
                $($ty.html_len() +)* $first.html_len()
            }

			fn to_html_with_buf(self, buf: &mut String, position: &mut Position, escape: bool, mark_branches: bool) {
                #[allow(non_snake_case)]
                let ($first, $($ty,)* ) = self;
                $first.to_html_with_buf(buf, position, escape, mark_branches);
                $($ty.to_html_with_buf(buf, position, escape, mark_branches));*
			}

			fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
				self,
				buf: &mut StreamBuilder, position: &mut Position, escape: bool, mark_branches: bool) where
				Self: Sized,
			{
                #[allow(non_snake_case)]
                let ($first, $($ty,)* ) = self;
                $first.to_html_async_with_buf::<OUT_OF_ORDER>(buf, position, escape, mark_branches);
                $($ty.to_html_async_with_buf::<OUT_OF_ORDER>(buf, position, escape, mark_branches));*
			}

			fn hydrate<const FROM_SERVER: bool>(self, cursor: &Cursor, position: &PositionState) -> Self::State {
                #[allow(non_snake_case)]
					let ($first, $($ty,)* ) = self;
					(
						$first.hydrate::<FROM_SERVER>(cursor, position),
						$($ty.hydrate::<FROM_SERVER>(cursor, position)),*
					)
			}

            async fn resolve(self) -> Self::AsyncOutput {
                #[allow(non_snake_case)]
                let ($first, $($ty,)*) = self;
                futures::join!(
                    $first.resolve(),
                    $($ty.resolve()),*
                )
            }

            fn dry_resolve(&mut self) {
                #[allow(non_snake_case)]
                let ($first, $($ty,)*) = self;
                $first.dry_resolve();
                $($ty.dry_resolve());*
            }
		}

		impl<$first, $($ty),*> ToTemplate for ($first, $($ty,)*)
		where
			$first: ToTemplate,
			$($ty: ToTemplate),*
		{
			const TEMPLATE: &'static str = str_from_buffer(&const_concat(&[
				$first::TEMPLATE, $($ty::TEMPLATE),*
			]));
			const CLASS: &'static str = str_from_buffer(&const_concat_with_separator(&[
				$first::CLASS, $($ty::CLASS),*
			], " "));
			const STYLE: &'static str = str_from_buffer(&const_concat_with_separator(&[
				$first::STYLE, $($ty::STYLE),*
			], ";"));

			fn to_template(buf: &mut String, class: &mut String, style: &mut String, inner_html: &mut String, position: &mut Position)  {
                $first ::to_template(buf, class, style, inner_html, position);
                $($ty::to_template(buf, class, style, inner_html, position));*;
			}
		}

		impl<$first, $($ty),*> Mountable for ($first, $($ty,)*) where
			$first: Mountable,
			$($ty: Mountable),*,

		{
			fn unmount(&mut self) {
                #[allow(non_snake_case)] // better macro performance
                let ($first, $($ty,)*) = self;
                $first.unmount();
                $($ty.unmount());*
			}

			fn mount(
				&mut self,
				parent: &crate::renderer::types::Element,
				marker: Option<&crate::renderer::types::Node>,
			) {
                #[allow(non_snake_case)] // better macro performance
                let ($first, $($ty,)*) = self;
                $first.mount(parent, marker);
                $($ty.mount(parent, marker));*
			}

			fn insert_before_this(&self,
				child: &mut dyn Mountable,
			) -> bool {
                #[allow(non_snake_case)] // better macro performance
                let ($first, $($ty,)*) = self;
                $first.insert_before_this(child)
                $(|| $ty.insert_before_this(child))*
			}
		}

        impl<$first, $($ty,)*> AddAnyAttr for ($first, $($ty,)*)
        where
			$first: AddAnyAttr,
			$($ty: AddAnyAttr),*,

        {
            type Output<SomeNewAttr: Attribute> = ($first::Output<SomeNewAttr::Cloneable>, $($ty::Output<SomeNewAttr::Cloneable>,)*);

            fn add_any_attr<NewAttr: Attribute>(
                self,
                attr: NewAttr,
            ) -> Self::Output<NewAttr>
            where
                Self::Output<NewAttr>: RenderHtml,
            {
                let shared = attr.into_cloneable();
                #[allow(non_snake_case)] // better macro performance
                let ($first, $($ty,)*) = self;
                ($first.add_any_attr(shared.clone()), $($ty.add_any_attr(shared.clone()),)*)
            }
        }
    };
}

impl_view_for_tuples!(A, B);
impl_view_for_tuples!(A, B, C);
impl_view_for_tuples!(A, B, C, D);
impl_view_for_tuples!(A, B, C, D, E);
impl_view_for_tuples!(A, B, C, D, E, F);
impl_view_for_tuples!(A, B, C, D, E, F, G);
impl_view_for_tuples!(A, B, C, D, E, F, G, H);
impl_view_for_tuples!(A, B, C, D, E, F, G, H, I);
impl_view_for_tuples!(A, B, C, D, E, F, G, H, I, J);
impl_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K);
impl_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
impl_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q);
impl_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R);
impl_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S);
impl_view_for_tuples!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T
);
impl_view_for_tuples!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U
);
impl_view_for_tuples!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V
);
impl_view_for_tuples!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W
);
impl_view_for_tuples!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X
);
impl_view_for_tuples!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y
);
impl_view_for_tuples!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y,
    Z
);
