/// A type-erased `AnyAttribute`.
pub mod any_attribute;
/// Types for ARIA attributes.
pub mod aria;
/// Types for custom attributes.
pub mod custom;
/// Traits to define global attribute methods on all HTML elements.
pub mod global;
mod key;
mod value;
use crate::{
    renderer::Renderer,
    view::{Position, ToTemplate},
};
pub use key::*;
use std::{fmt::Debug, future::Future, marker::PhantomData};
pub use value::*;

/// Defines an attribute: anything that can modify an element.
pub trait Attribute<R: Renderer>: NextAttribute<R> + Send {
    /// The minimum length of this attribute in HTML.
    const MIN_LENGTH: usize;

    /// The state that should be retained between building and rebuilding.
    type State;
    /// The type once all async data have loaded.
    type AsyncOutput: Attribute<R>;
    /// An equivalent to this attribute that can be cloned to be shared across elements.
    type Cloneable: Attribute<R> + Clone;
    /// An equivalent to this attribute that can be cloned to be shared across elements, and
    /// captures no references shorter than `'static`.
    type CloneableOwned: Attribute<R> + Clone + 'static;

    /// An approximation of the actual length of this attribute in HTML.
    fn html_len(&self) -> usize;

    /// Renders the attribute to HTML.
    ///
    /// This separates a general buffer for attribute values from the `class` and `style`
    /// attributes, so that multiple classes or styles can be combined, and also allows for an
    /// `inner_html` attribute that sets the child HTML instead of an attribute.
    fn to_html(
        self,
        buf: &mut String,
        class: &mut String,
        style: &mut String,
        inner_html: &mut String,
    );

    /// Adds interactivity as necessary, given DOM nodes that were created from HTML that has
    /// either been rendered on the server, or cloned for a `<template>`.
    fn hydrate<const FROM_SERVER: bool>(self, el: &R::Element) -> Self::State;

    /// Adds this attribute to the element during client-side rendering.
    fn build(self, el: &R::Element) -> Self::State;

    /// Applies a new value for the attribute.
    fn rebuild(self, state: &mut Self::State);

    /// Converts this attribute into an equivalent that can be cloned.
    fn into_cloneable(self) -> Self::Cloneable;

    /// Converts this attributes into an equivalent that can be cloned and is `'static`.
    fn into_cloneable_owned(self) -> Self::CloneableOwned;

    /// “Runs” the attribute without other side effects. For primitive types, this is a no-op. For
    /// reactive types, this can be used to gather data about reactivity or about asynchronous data
    /// that needs to be loaded.
    fn dry_resolve(&mut self);

    /// “Resolves” this into a type that is not waiting for any asynchronous data.
    fn resolve(self) -> impl Future<Output = Self::AsyncOutput> + Send;
}

/// Adds another attribute to this one, returning a new attribute.
///
/// This is typically achieved by creating or extending a tuple of attributes.
pub trait NextAttribute<R: Renderer> {
    /// The type of the new, combined attribute.
    type Output<NewAttr: Attribute<R>>: Attribute<R>;

    /// Adds a new attribute.
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
    type AsyncOutput = ();
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

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {}
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

/// An attribute with a key and value.
#[derive(Debug)]
pub struct Attr<K, V, R>(pub K, pub V, pub PhantomData<R>)
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
    type AsyncOutput = Attr<K, V::AsyncOutput, R>;
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

    fn dry_resolve(&mut self) {
        self.1.dry_resolve();
    }

    async fn resolve(self) -> Self::AsyncOutput {
        Attr(self.0, self.1.resolve().await, PhantomData)
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

			type AsyncOutput = ($first::AsyncOutput, $($ty::AsyncOutput,)*);
			type State = ($first::State, $($ty::State,)*);
            type Cloneable = ($first::Cloneable, $($ty::Cloneable,)*);
            type CloneableOwned = ($first::CloneableOwned, $($ty::CloneableOwned,)*);

            fn html_len(&self) -> usize {
                #[allow(non_snake_case)]
                let ($first, $($ty,)*) = self;
                $first.html_len() $(+ $ty.html_len())*
            }

			fn to_html(self, buf: &mut String, class: &mut String, style: &mut String, inner_html: &mut String,) {
                #[allow(non_snake_case)]
					let ($first, $($ty,)* ) = self;
					$first.to_html(buf, class, style, inner_html);
					$($ty.to_html(buf, class, style, inner_html));*
			}

			fn hydrate<const FROM_SERVER: bool>(self, el: &Rndr::Element) -> Self::State {
                #[allow(non_snake_case)]
					let ($first, $($ty,)* ) = self;
					(
						$first.hydrate::<FROM_SERVER>(el),
						$($ty.hydrate::<FROM_SERVER>(el)),*
					)
			}

            fn build(self, el: &Rndr::Element) -> Self::State {
                #[allow(non_snake_case)]
					let ($first, $($ty,)*) = self;
                    (
                        $first.build(el),
                        $($ty.build(el)),*
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

            fn into_cloneable(self) -> Self::Cloneable {
                #[allow(non_snake_case)]
                let ($first, $($ty,)*) = self;
                (
                    $first.into_cloneable(),
                    $($ty.into_cloneable()),*
                )
            }

            fn into_cloneable_owned(self) -> Self::CloneableOwned {
                #[allow(non_snake_case)]
                let ($first, $($ty,)*) = self;
                (
                    $first.into_cloneable_owned(),
                    $($ty.into_cloneable_owned()),*
                )
            }

            fn dry_resolve(&mut self) {
                #[allow(non_snake_case)]
                let ($first, $($ty,)*) = self;
                $first.dry_resolve();
                $($ty.dry_resolve());*
            }

            async fn resolve(self) -> Self::AsyncOutput {
                #[allow(non_snake_case)]
                let ($first, $($ty,)*) = self;
                futures::join!(
                    $first.resolve(),
                    $($ty.resolve()),*
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

			type AsyncOutput = ($first::AsyncOutput, $($ty::AsyncOutput,)*);
			type State = ($first::State, $($ty::State,)*);
            type Cloneable = ($first::Cloneable, $($ty::Cloneable,)*);
            type CloneableOwned = ($first::CloneableOwned, $($ty::CloneableOwned,)*);

            fn html_len(&self) -> usize {
                #[allow(non_snake_case)]
                let ($first, $($ty,)*) = self;
                $first.html_len() $(+ $ty.html_len())*
            }

			fn to_html(self, buf: &mut String, class: &mut String, style: &mut String, inner_html: &mut String,) {
                #[allow(non_snake_case)]
                let ($first, $($ty,)* ) = self;
                $first.to_html(buf, class, style, inner_html);
                $($ty.to_html(buf, class, style, inner_html));*
			}

			fn hydrate<const FROM_SERVER: bool>(self, el: &Rndr::Element) -> Self::State {
                #[allow(non_snake_case)]
                let ($first, $($ty,)* ) = self;
                (
                    $first.hydrate::<FROM_SERVER>(el),
                    $($ty.hydrate::<FROM_SERVER>(el)),*
                )
			}

            fn build(self, el: &Rndr::Element) -> Self::State {
                #[allow(non_snake_case)]
                let ($first, $($ty,)*) = self;
                (
                    $first.build(el),
                    $($ty.build(el)),*
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

            fn into_cloneable(self) -> Self::Cloneable {
                #[allow(non_snake_case)]
                let ($first, $($ty,)*) = self;
                (
                    $first.into_cloneable(),
                    $($ty.into_cloneable()),*
                )
            }

            fn into_cloneable_owned(self) -> Self::CloneableOwned {
                #[allow(non_snake_case)]
                let ($first, $($ty,)*) = self;
                (
                    $first.into_cloneable_owned(),
                    $($ty.into_cloneable_owned()),*
                )
            }

            fn dry_resolve(&mut self) {
                #[allow(non_snake_case)]
                let ($first, $($ty,)*) = self;
                $first.dry_resolve();
                $($ty.dry_resolve());*
            }

            async fn resolve(self) -> Self::AsyncOutput {
                #[allow(non_snake_case)]
                let ($first, $($ty,)*) = self;
                futures::join!(
                    $first.resolve(),
                    $($ty.resolve()),*
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

    type AsyncOutput = (A::AsyncOutput,);
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

    fn dry_resolve(&mut self) {
        self.0.dry_resolve();
    }

    async fn resolve(self) -> Self::AsyncOutput {
        (self.0.resolve().await,)
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
