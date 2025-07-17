use either_of::*;
use std::{future::Future, marker::PhantomData};
use tachys::view::any_view::{AnyView, IntoAny};

pub trait ChooseView
where
    Self: Send + Clone + 'static,
{
    type Data: Send + 'static;

    fn choose(self, data: Self::Data) -> impl Future<Output = AnyView>;

    fn preload(&self) -> impl Future<Output = ()>;

    fn data(&self) -> Self::Data;
}

impl<F, View> ChooseView for F
where
    F: Fn() -> View + Send + Clone + 'static,
    View: IntoAny,
{
    type Data = ();

    async fn choose(self, _data: ()) -> AnyView {
        self().into_any()
    }

    async fn preload(&self) {}

    fn data(&self) -> Self::Data {}
}

impl<T> ChooseView for Lazy<T>
where
    T: LazyRoute,
{
    type Data = T;

    async fn choose(self, data: T) -> AnyView {
        T::view(data).await
    }

    async fn preload(&self) {
        T::preload().await;
    }

    fn data(&self) -> Self::Data {
        T::data()
    }
}

pub trait LazyRoute: Send + 'static {
    fn data() -> Self;

    fn view(this: Self) -> impl Future<Output = AnyView>;

    fn preload() -> impl Future<Output = ()> {
        async {}
    }
}

#[derive(Debug)]
pub struct Lazy<T> {
    ty: PhantomData<T>,
}

impl<T> Clone for Lazy<T> {
    fn clone(&self) -> Self {
        Self { ty: self.ty }
    }
}

impl<T> Lazy<T> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T> Default for Lazy<T> {
    fn default() -> Self {
        Self {
            ty: Default::default(),
        }
    }
}

impl ChooseView for () {
    type Data = ();

    async fn choose(self, _data: ()) -> AnyView {
        ().into_any()
    }

    async fn preload(&self) {}

    fn data(&self) -> Self::Data {}
}

impl<A, B> ChooseView for Either<A, B>
where
    A: ChooseView,
    B: ChooseView,
{
    type Data = Either<A::Data, B::Data>;

    async fn choose(self, data: Self::Data) -> AnyView {
        match (self, data) {
            (Either::Left(f), Either::Left(d)) => f.choose(d).await.into_any(),
            (Either::Right(f), Either::Right(d)) => {
                f.choose(d).await.into_any()
            }
            _ => unreachable!(),
        }
    }

    async fn preload(&self) {
        match self {
            Either::Left(f) => f.preload().await,
            Either::Right(f) => f.preload().await,
        }
    }

    fn data(&self) -> Self::Data {
        match self {
            Either::Left(f) => Either::Left(f.data()),
            Either::Right(f) => Either::Right(f.data()),
        }
    }
}

macro_rules! tuples {
    ($either:ident => $($ty:ident),*) => {
        impl<$($ty,)*> ChooseView for $either<$($ty,)*>
        where
            $($ty: ChooseView,)*
        {
            type Data = $either<$($ty::Data),*>;

            async fn choose(self, data: Self::Data) -> AnyView {
                match (self, data) {
                    $(
                        ($either::$ty(f), $either::$ty(d)) => f.choose(d).await.into_any(),
                    )*
                    _ => unreachable!()
                }
            }

            async fn preload(&self) {
                match self {
                    $($either::$ty(f) => f.preload().await,)*
                }
            }

            fn data(&self) -> Self::Data {
                match self {
                    $($either::$ty(f) => $either::$ty(f.data()),)*
                }
            }
        }
    };
}

tuples!(EitherOf3 => A, B, C);
tuples!(EitherOf4 => A, B, C, D);
tuples!(EitherOf5 => A, B, C, D, E);
tuples!(EitherOf6 => A, B, C, D, E, F);
tuples!(EitherOf7 => A, B, C, D, E, F, G);
tuples!(EitherOf8 => A, B, C, D, E, F, G, H);
tuples!(EitherOf9 => A, B, C, D, E, F, G, H, I);
tuples!(EitherOf10 => A, B, C, D, E, F, G, H, I, J);
tuples!(EitherOf11 => A, B, C, D, E, F, G, H, I, J, K);
tuples!(EitherOf12 => A, B, C, D, E, F, G, H, I, J, K, L);
tuples!(EitherOf13 => A, B, C, D, E, F, G, H, I, J, K, L, M);
tuples!(EitherOf14 => A, B, C, D, E, F, G, H, I, J, K, L, M, N);
tuples!(EitherOf15 => A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
tuples!(EitherOf16 => A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);

/// A version of [`IntoMaybeErased`] for the [`ChooseView`] trait.
pub trait IntoChooseViewMaybeErased {
    /// The type of the erased view.
    type Output: IntoChooseViewMaybeErased;

    /// Erase the type of the view.
    fn into_maybe_erased(self) -> Self::Output;
}

impl<T> IntoChooseViewMaybeErased for T
where
    T: ChooseView + Send + Clone + 'static,
{
    #[cfg(erase_components)]
    type Output = crate::matching::any_choose_view::AnyChooseView;

    #[cfg(not(erase_components))]
    type Output = Self;

    fn into_maybe_erased(self) -> Self::Output {
        #[cfg(erase_components)]
        {
            crate::matching::any_choose_view::AnyChooseView::new(self)
        }
        #[cfg(not(erase_components))]
        {
            self
        }
    }
}
