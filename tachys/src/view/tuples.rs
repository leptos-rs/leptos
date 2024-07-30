use super::{
    Mountable, Position, PositionState, Render, RenderHtml, Renderer,
    ToTemplate,
};
use crate::{
    html::attribute::Attribute,
    hydration::Cursor,
    view::{add_attr::AddAnyAttr, StreamBuilder},
};
use const_str_slice_concat::{
    const_concat, const_concat_with_separator, str_from_buffer,
};

impl<R: Renderer> Render<R> for () {
    type State = ();

    fn build(self) -> Self::State {}

    fn rebuild(self, _state: &mut Self::State) {}
}

impl<R> RenderHtml<R> for ()
where
    R: Renderer,
{
    type AsyncOutput = ();

    const MIN_LENGTH: usize = 0;

    fn to_html_with_buf(self, _buf: &mut String, _position: &mut Position) {}

    fn hydrate<const FROM_SERVER: bool>(
        self,
        _cursor: &Cursor<R>,
        _position: &PositionState,
    ) -> Self::State {
    }

    async fn resolve(self) -> Self::AsyncOutput {}

    fn dry_resolve(&mut self) {}
}

impl<Rndr> AddAnyAttr<Rndr> for ()
where
    Rndr: Renderer,
{
    type Output<SomeNewAttr: Attribute<Rndr>> = ();

    fn add_any_attr<NewAttr: Attribute<Rndr>>(
        self,
        _attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<Rndr>,
    {
        ().add_any_attr(_attr)
    }
}

impl<R: Renderer> Mountable<R> for () {
    fn unmount(&mut self) {}

    fn mount(&mut self, _parent: &R::Element, _marker: Option<&R::Node>) {}

    fn insert_before_this(&self, _child: &mut dyn Mountable<R>) -> bool {
        false
    }
}

impl ToTemplate for () {
    const TEMPLATE: &'static str = "";

    fn to_template(
        _buf: &mut String,
        _class: &mut String,
        _style: &mut String,
        _inner_html: &mut String,
        _position: &mut Position,
    ) {
    }
}

impl<A: Render<R>, R: Renderer> Render<R> for (A,) {
    type State = A::State;

    fn build(self) -> Self::State {
        self.0.build()
    }

    fn rebuild(self, state: &mut Self::State) {
        self.0.rebuild(state)
    }
}

impl<A, R> RenderHtml<R> for (A,)
where
    A: RenderHtml<R>,
    R: Renderer,
{
    type AsyncOutput = (A::AsyncOutput,);

    const MIN_LENGTH: usize = A::MIN_LENGTH;

    fn html_len(&self) -> usize {
        self.0.html_len()
    }

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        self.0.to_html_with_buf(buf, position);
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
    ) where
        Self: Sized,
    {
        self.0.to_html_async_with_buf::<OUT_OF_ORDER>(buf, position);
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
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

impl<A, Rndr> AddAnyAttr<Rndr> for (A,)
where
    A: AddAnyAttr<Rndr>,
    Rndr: Renderer,
{
    type Output<SomeNewAttr: Attribute<Rndr>> = (A::Output<SomeNewAttr>,);

    fn add_any_attr<NewAttr: Attribute<Rndr>>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<Rndr>,
    {
        (self.0.add_any_attr(attr),)
    }
}

macro_rules! impl_view_for_tuples {
	($first:ident, $($ty:ident),* $(,)?) => {
		impl<$first, $($ty),*, Rndr> Render<Rndr> for ($first, $($ty,)*)
		where
			$first: Render<Rndr>,
			$($ty: Render<Rndr>),*,
			Rndr: Renderer
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

		impl<$first, $($ty),*, Rndr> RenderHtml<Rndr> for ($first, $($ty,)*)
		where
			$first: RenderHtml<Rndr>,
			$($ty: RenderHtml<Rndr>),*,
			Rndr: Renderer,
		{
            type AsyncOutput = ($first::AsyncOutput, $($ty::AsyncOutput,)*);

            const MIN_LENGTH: usize = $first::MIN_LENGTH $(+ $ty::MIN_LENGTH)*;

            #[inline(always)]
            fn html_len(&self) -> usize {
                #[allow(non_snake_case)]
			    let ($first, $($ty,)* ) = self;
                $($ty.html_len() +)* $first.html_len()
            }

			fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
                #[allow(non_snake_case)]
                let ($first, $($ty,)* ) = self;
                $first.to_html_with_buf(buf, position);
                $($ty.to_html_with_buf(buf, position));*
			}

			fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
				self,
				buf: &mut StreamBuilder,
				position: &mut Position,
			) where
				Self: Sized,
			{
                #[allow(non_snake_case)]
                let ($first, $($ty,)* ) = self;
                $first.to_html_async_with_buf::<OUT_OF_ORDER>(buf, position);
                $($ty.to_html_async_with_buf::<OUT_OF_ORDER>(buf, position));*
			}

			fn hydrate<const FROM_SERVER: bool>(self, cursor: &Cursor<Rndr>, position: &PositionState) -> Self::State {
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

		impl<$first, $($ty),*, Rndr> Mountable<Rndr> for ($first, $($ty,)*) where
			$first: Mountable<Rndr>,
			$($ty: Mountable<Rndr>),*,
			Rndr: Renderer
		{
			fn unmount(&mut self) {
                #[allow(non_snake_case)] // better macro performance
                let ($first, $($ty,)*) = self;
                $first.unmount();
                $($ty.unmount());*
			}

			fn mount(
				&mut self,
				parent: &Rndr::Element,
				marker: Option<&Rndr::Node>,
			) {
                #[allow(non_snake_case)] // better macro performance
                let ($first, $($ty,)*) = self;
                $first.mount(parent, marker);
                $($ty.mount(parent, marker));*
			}

			fn insert_before_this(&self,
				child: &mut dyn Mountable<Rndr>,
			) -> bool {
                #[allow(non_snake_case)] // better macro performance
                let ($first, $($ty,)*) = self;
                $first.insert_before_this(child)
                $(|| $ty.insert_before_this(child))*
			}
		}

        impl<$first, $($ty,)* Rndr> AddAnyAttr<Rndr> for ($first, $($ty,)*)
        where
			$first: AddAnyAttr<Rndr>,
			$($ty: AddAnyAttr<Rndr>),*,
            Rndr: Renderer,
        {
            type Output<SomeNewAttr: Attribute<Rndr>> = ($first::Output<SomeNewAttr::Cloneable>, $($ty::Output<SomeNewAttr::Cloneable>,)*);

            fn add_any_attr<NewAttr: Attribute<Rndr>>(
                self,
                attr: NewAttr,
            ) -> Self::Output<NewAttr>
            where
                Self::Output<NewAttr>: RenderHtml<Rndr>,
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
