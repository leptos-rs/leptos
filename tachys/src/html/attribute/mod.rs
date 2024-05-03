pub mod any_attribute;
pub mod aria;
pub mod custom;
pub mod global;
mod key;
mod value;
use crate::{
    renderer::Renderer,
    view::{Position, ToTemplate},
};
pub use key::*;
use std::{fmt::Debug, marker::PhantomData};
pub use value::*;

pub trait Attribute<R: Renderer>: NextAttribute<R> + Send {
    const MIN_LENGTH: usize;

    type State;
    type Cloneable: Attribute<R> + Clone;
    type CloneableOwned: Attribute<R> + Clone + 'static;

    fn html_len(&self) -> usize;

    fn to_html(
        self,
        buf: &mut String,
        class: &mut String,
        style: &mut String,
        inner_html: &mut String,
    );

    fn hydrate<const FROM_SERVER: bool>(self, el: &R::Element) -> Self::State;

    fn build(self, el: &R::Element) -> Self::State;

    fn rebuild(self, state: &mut Self::State);

    fn into_cloneable(self) -> Self::Cloneable;

    fn into_cloneable_owned(self) -> Self::CloneableOwned;
}

pub trait NextAttribute<R: Renderer> {
    type Output<NewAttr: Attribute<R>>: Attribute<R>;

    fn add_any_attr<NewAttr: Attribute<R>>(
        self,
        new_attr: NewAttr,
    ) -> Self::Output<NewAttr>;
}

impl<R> Attribute<R> for ()
where
    R: Renderer,
{
    const MIN_LENGTH: usize = 0;

    type State = ();
    type Cloneable = ();
    type CloneableOwned = ();

    fn html_len(&self) -> usize {
        0
    }

    fn to_html(
        self,
        _buf: &mut String,
        _class: &mut String,
        _style: &mut String,
        _inner_html: &mut String,
    ) {
    }

    fn hydrate<const FROM_SERVER: bool>(self, _el: &R::Element) -> Self::State {
    }

    fn build(self, _el: &R::Element) -> Self::State {}

    fn rebuild(self, _state: &mut Self::State) {}

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::Cloneable {
        self
    }
}

impl<R> NextAttribute<R> for ()
where
    R: Renderer,
{
    type Output<NewAttr: Attribute<R>> = (NewAttr,);

    fn add_any_attr<NewAttr: Attribute<R>>(
        self,
        new_attr: NewAttr,
    ) -> Self::Output<NewAttr> {
        (new_attr,)
    }
}

#[derive(Debug)]
pub struct Attr<K, V, R>(pub K, pub V, PhantomData<R>)
where
    K: AttributeKey,
    V: AttributeValue<R>,
    R: Renderer;

impl<K, V, R> Clone for Attr<K, V, R>
where
    K: AttributeKey,
    V: AttributeValue<R> + Clone,
    R: Renderer,
{
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone(), PhantomData)
    }
}

impl<K, V, R> ToTemplate for Attr<K, V, R>
where
    K: AttributeKey,
    V: AttributeValue<R>,
    R: Renderer,
{
    fn to_template(
        buf: &mut String,
        _class: &mut String,
        _style: &mut String,
        _inner_html: &mut String,
        _position: &mut Position,
    ) {
        V::to_template(K::KEY, buf);
    }
}

impl<K, V, R> Attribute<R> for Attr<K, V, R>
where
    K: AttributeKey + Send,
    V: AttributeValue<R> + Send,
    R: Renderer,
{
    const MIN_LENGTH: usize = 0;

    type State = V::State;
    type Cloneable = Attr<K, V::Cloneable, R>;
    type CloneableOwned = Attr<K, V::CloneableOwned, R>;

    fn html_len(&self) -> usize {
        K::KEY.len() + 3 + self.1.html_len()
    }

    fn to_html(
        self,
        buf: &mut String,
        _class: &mut String,
        _style: &mut String,
        _inner_html: &mut String,
    ) {
        self.1.to_html(K::KEY, buf);
    }

    fn hydrate<const FROM_SERVER: bool>(self, el: &R::Element) -> Self::State {
        self.1.hydrate::<FROM_SERVER>(K::KEY, el)
    }

    fn build(self, el: &R::Element) -> Self::State {
        V::build(self.1, el, K::KEY)
    }

    fn rebuild(self, state: &mut Self::State) {
        V::rebuild(self.1, K::KEY, state);
    }

    fn into_cloneable(self) -> Self::Cloneable {
        Attr(self.0, self.1.into_cloneable(), PhantomData)
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        Attr(self.0, self.1.into_cloneable_owned(), PhantomData)
    }
}

impl<K, V, R> NextAttribute<R> for Attr<K, V, R>
where
    K: AttributeKey,
    V: AttributeValue<R>,
    R: Renderer,
{
    type Output<NewAttr: Attribute<R>> = (Self, NewAttr);

    fn add_any_attr<NewAttr: Attribute<R>>(
        self,
        new_attr: NewAttr,
    ) -> Self::Output<NewAttr> {
        (self, new_attr)
    }
}

macro_rules! impl_attr_for_tuples {
	($first:ident, $($ty:ident),* $(,)?) => {
		impl<$first, $($ty),*, Rndr> Attribute<Rndr> for ($first, $($ty,)*)
		where
			$first: Attribute<Rndr>,
			$($ty: Attribute<Rndr>),*,
            Rndr: Renderer
		{
            const MIN_LENGTH: usize = $first::MIN_LENGTH $(+ $ty::MIN_LENGTH)*;

			type State = ($first::State, $($ty::State,)*);
            type Cloneable = ($first::Cloneable, $($ty::Cloneable,)*);
            type CloneableOwned = ($first::CloneableOwned, $($ty::CloneableOwned,)*);

            fn html_len(&self) -> usize {
                #[allow(non_snake_case)]
                let ($first, $($ty,)*) = self;
                $first.html_len() $(+ $ty.html_len())*
            }

			fn to_html(self, buf: &mut String, class: &mut String, style: &mut String, inner_html: &mut String,) {
				paste::paste! {
					let ([<$first:lower>], $([<$ty:lower>],)* ) = self;
					[<$first:lower>].to_html(buf, class, style, inner_html);
					$([<$ty:lower>].to_html(buf, class, style, inner_html));*
				}
			}

			fn hydrate<const FROM_SERVER: bool>(self, el: &Rndr::Element) -> Self::State {
				paste::paste! {
					let ([<$first:lower>], $([<$ty:lower>],)* ) = self;
					(
						[<$first:lower>].hydrate::<FROM_SERVER>(el),
						$([<$ty:lower>].hydrate::<FROM_SERVER>(el)),*
					)
				}
			}

            fn build(self, el: &Rndr::Element) -> Self::State {
				paste::paste! {
					let ([<$first:lower>], $([<$ty:lower>],)*) = self;
                    (
                        [<$first:lower>].build(el),
                        $([<$ty:lower>].build(el)),*
                    )
				}
			}

			fn rebuild(self, state: &mut Self::State) {
				paste::paste! {
					let ([<$first:lower>], $([<$ty:lower>],)*) = self;
					let ([<view_ $first:lower>], $([<view_ $ty:lower>],)*) = state;
					[<$first:lower>].rebuild([<view_ $first:lower>]);
					$([<$ty:lower>].rebuild([<view_ $ty:lower>]));*
				}
			}

            fn into_cloneable(self) -> Self::Cloneable {
                let ($first, $($ty,)*) = self;
                (
                    $first.into_cloneable(),
                    $($ty.into_cloneable()),*
                )
            }

            fn into_cloneable_owned(self) -> Self::CloneableOwned {
                let ($first, $($ty,)*) = self;
                (
                    $first.into_cloneable_owned(),
                    $($ty.into_cloneable_owned()),*
                )
            }
        }

		impl<$first, $($ty),*, Rndr> NextAttribute<Rndr> for ($first, $($ty,)*)
		where
			$first: Attribute<Rndr>,
			$($ty: Attribute<Rndr>),*,
            Rndr: Renderer
        {
            type Output<NewAttr: Attribute<Rndr>> = ($first, $($ty,)* NewAttr);

            fn add_any_attr<NewAttr: Attribute<Rndr>>(
                self,
                new_attr: NewAttr,
            ) -> Self::Output<NewAttr> {
                #[allow(non_snake_case)]
                let ($first, $($ty,)*) = self;
                ($first, $($ty,)* new_attr)
            }
		}
	};
}

macro_rules! impl_attr_for_tuples_truncate_additional {
	($first:ident, $($ty:ident),* $(,)?) => {
		impl<$first, $($ty),*, Rndr> Attribute<Rndr> for ($first, $($ty,)*)
		where
			$first: Attribute<Rndr>,
			$($ty: Attribute<Rndr>),*,
            Rndr: Renderer
		{
            const MIN_LENGTH: usize = $first::MIN_LENGTH $(+ $ty::MIN_LENGTH)*;

			type State = ($first::State, $($ty::State,)*);
            type Cloneable = ($first::Cloneable, $($ty::Cloneable,)*);
            type CloneableOwned = ($first::CloneableOwned, $($ty::CloneableOwned,)*);

            fn html_len(&self) -> usize {
                #[allow(non_snake_case)]
                let ($first, $($ty,)*) = self;
                $first.html_len() $(+ $ty.html_len())*
            }

			fn to_html(self, buf: &mut String, class: &mut String, style: &mut String, inner_html: &mut String,) {
				paste::paste! {
					let ([<$first:lower>], $([<$ty:lower>],)* ) = self;
					[<$first:lower>].to_html(buf, class, style, inner_html);
					$([<$ty:lower>].to_html(buf, class, style, inner_html));*
				}
			}

			fn hydrate<const FROM_SERVER: bool>(self, el: &Rndr::Element) -> Self::State {
				paste::paste! {
					let ([<$first:lower>], $([<$ty:lower>],)* ) = self;
					(
						[<$first:lower>].hydrate::<FROM_SERVER>(el),
						$([<$ty:lower>].hydrate::<FROM_SERVER>(el)),*
					)
				}
			}

            fn build(self, el: &Rndr::Element) -> Self::State {
				paste::paste! {
					let ([<$first:lower>], $([<$ty:lower>],)*) = self;
                    (
                        [<$first:lower>].build(el),
                        $([<$ty:lower>].build(el)),*
                    )
				}
			}

			fn rebuild(self, state: &mut Self::State) {
				paste::paste! {
					let ([<$first:lower>], $([<$ty:lower>],)*) = self;
					let ([<view_ $first:lower>], $([<view_ $ty:lower>],)*) = state;
					[<$first:lower>].rebuild([<view_ $first:lower>]);
					$([<$ty:lower>].rebuild([<view_ $ty:lower>]));*
				}
			}

            fn into_cloneable(self) -> Self::Cloneable {
                let ($first, $($ty,)*) = self;
                (
                    $first.into_cloneable(),
                    $($ty.into_cloneable()),*
                )
            }

            fn into_cloneable_owned(self) -> Self::CloneableOwned {
                let ($first, $($ty,)*) = self;
                (
                    $first.into_cloneable_owned(),
                    $($ty.into_cloneable_owned()),*
                )
            }
        }

		impl<$first, $($ty),*, Rndr> NextAttribute<Rndr> for ($first, $($ty,)*)
		where
			$first: Attribute<Rndr>,
			$($ty: Attribute<Rndr>),*,
            Rndr: Renderer
        {
            type Output<NewAttr: Attribute<Rndr>> = ($first, $($ty,)*);

            fn add_any_attr<NewAttr: Attribute<Rndr>>(
                self,
                _new_attr: NewAttr,
            ) -> Self::Output<NewAttr> {
                todo!("adding more than 26 attributes is not supported");
                //($first, $($ty,)*)
            }
		}
	};
}

impl<A, Rndr> Attribute<Rndr> for (A,)
where
    A: Attribute<Rndr>,
    Rndr: Renderer,
{
    const MIN_LENGTH: usize = A::MIN_LENGTH;

    type State = A::State;
    type Cloneable = (A::Cloneable,);
    type CloneableOwned = (A::CloneableOwned,);

    fn html_len(&self) -> usize {
        self.0.html_len()
    }

    fn to_html(
        self,
        buf: &mut String,
        class: &mut String,
        style: &mut String,
        inner_html: &mut String,
    ) {
        self.0.to_html(buf, class, style, inner_html);
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &Rndr::Element,
    ) -> Self::State {
        self.0.hydrate::<FROM_SERVER>(el)
    }

    fn build(self, el: &Rndr::Element) -> Self::State {
        self.0.build(el)
    }

    fn rebuild(self, state: &mut Self::State) {
        self.0.rebuild(state);
    }

    fn into_cloneable(self) -> Self::Cloneable {
        (self.0.into_cloneable(),)
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        (self.0.into_cloneable_owned(),)
    }
}

impl<A, Rndr> NextAttribute<Rndr> for (A,)
where
    A: Attribute<Rndr>,
    Rndr: Renderer,
{
    type Output<NewAttr: Attribute<Rndr>> = (A, NewAttr);

    fn add_any_attr<NewAttr: Attribute<Rndr>>(
        self,
        new_attr: NewAttr,
    ) -> Self::Output<NewAttr> {
        (self.0, new_attr)
    }
}

impl_attr_for_tuples!(A, B);
impl_attr_for_tuples!(A, B, C);
impl_attr_for_tuples!(A, B, C, D);
impl_attr_for_tuples!(A, B, C, D, E);
impl_attr_for_tuples!(A, B, C, D, E, F);
impl_attr_for_tuples!(A, B, C, D, E, F, G);
impl_attr_for_tuples!(A, B, C, D, E, F, G, H);
impl_attr_for_tuples!(A, B, C, D, E, F, G, H, I);
impl_attr_for_tuples!(A, B, C, D, E, F, G, H, I, J);
impl_attr_for_tuples!(A, B, C, D, E, F, G, H, I, J, K);
impl_attr_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_attr_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_attr_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_attr_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_attr_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
impl_attr_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q);
impl_attr_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R);
impl_attr_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S);
impl_attr_for_tuples!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T
);
impl_attr_for_tuples!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U
);
impl_attr_for_tuples!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V
);
impl_attr_for_tuples!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W
);
impl_attr_for_tuples!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X
);
impl_attr_for_tuples!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y
);
impl_attr_for_tuples_truncate_additional!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y,
    Z
);
