use either_of::*;
use std::{future::Future, marker::PhantomData};
use tachys::view::{any_view::AnyView, Render};

pub trait ChooseView
where
    Self: Send + Clone + 'static,
{
    type Output;

    fn choose(self) -> impl Future<Output = Self::Output>;

    fn preload(&self) -> impl Future<Output = ()>;
}

impl<F, View> ChooseView for F
where
    F: Fn() -> View + Send + Clone + 'static,
    View: Render + Send,
{
    type Output = View;

    async fn choose(self) -> Self::Output {
        self()
    }

    async fn preload(&self) {}
}

impl<T> ChooseView for Lazy<T>
where
    T: LazyRoute,
{
    type Output = AnyView;

    async fn choose(self) -> Self::Output {
        T::data().view().await
    }

    async fn preload(&self) {
        T::data().view().await;
    }
}

pub trait LazyRoute: Send + 'static {
    fn data() -> Self;

    fn view(self) -> impl Future<Output = AnyView>;
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
    type Output = ();

    async fn choose(self) -> Self::Output {}

    async fn preload(&self) {}
}

impl<A, B> ChooseView for Either<A, B>
where
    A: ChooseView,
    B: ChooseView,
{
    type Output = Either<A::Output, B::Output>;

    async fn choose(self) -> Self::Output {
        match self {
            Either::Left(f) => Either::Left(f.choose().await),
            Either::Right(f) => Either::Right(f.choose().await),
        }
    }

    async fn preload(&self) {
        match self {
            Either::Left(f) => f.preload().await,
            Either::Right(f) => f.preload().await,
        }
    }
}

macro_rules! tuples {
    ($either:ident => $($ty:ident),*) => {
        impl<$($ty,)*> ChooseView for $either<$($ty,)*>
        where
            $($ty: ChooseView,)*
        {
            type Output = $either<$($ty::Output,)*>;

            async fn choose(self ) -> Self::Output {
                match self {
                    $($either::$ty(f) => $either::$ty(f.choose().await),)*
                }
            }

            async fn preload(&self) {
                match self {
                    $($either::$ty(f) => f.preload().await,)*
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
