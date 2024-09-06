use either_of::*;
use std::{future::Future, marker::PhantomData};
use tachys::{
    renderer::Renderer,
    view::any_view::{AnyView, IntoAny},
};

pub trait ChooseView<R>
where
    Self: Send + Clone + 'static,
    R: Renderer + 'static,
{
    fn choose(self) -> impl Future<Output = AnyView<R>>;

    fn preload(&self) -> impl Future<Output = ()>;
}

impl<F, View, R> ChooseView<R> for F
where
    F: Fn() -> View + Send + Clone + 'static,
    View: IntoAny<R>,
    R: Renderer + 'static,
{
    async fn choose(self) -> AnyView<R> {
        self().into_any()
    }

    async fn preload(&self) {}
}

impl<T, R> ChooseView<R> for Lazy<T>
where
    T: LazyRoute<R>,
    R: Renderer + 'static,
{
    async fn choose(self) -> AnyView<R> {
        T::data().view().await.into_any()
    }

    async fn preload(&self) {
        T::data().view().await;
    }
}

pub trait LazyRoute<R>: Send + 'static
where
    R: Renderer,
{
    fn data() -> Self;

    fn view(self) -> impl Future<Output = AnyView<R>>;
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

impl<R> ChooseView<R> for ()
where
    R: Renderer + 'static,
{
    async fn choose(self) -> AnyView<R> { ().into_any() }

    async fn preload(&self) {}
}

impl<A, B, Rndr> ChooseView<Rndr> for Either<A, B>
where
    A: ChooseView<Rndr>,
    B: ChooseView<Rndr>,
    Rndr: Renderer + 'static,
{
    async fn choose(self) -> AnyView<Rndr> {
        match self {
            Either::Left(f) => f.choose().await.into_any(),
            Either::Right(f) => f.choose().await.into_any()
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
        impl<$($ty,)* Rndr> ChooseView<Rndr> for $either<$($ty,)*>
        where
            $($ty: ChooseView<Rndr>,)*
            Rndr: Renderer + 'static,
        {
            async fn choose(self ) -> AnyView<Rndr> {
                match self {
                    $($either::$ty(f) => f.choose().await.into_any(),)*
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
