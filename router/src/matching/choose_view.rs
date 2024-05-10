use either_of::*;
use std::{future::Future, marker::PhantomData};
use tachys::{
    renderer::Renderer,
    view::{any_view::AnyView, Render},
};

pub trait ChooseView<R>
where
    Self: Send + 'static,
    R: Renderer + 'static,
{
    type Output;

    fn choose(self) -> impl Future<Output = Self::Output>;
}

impl<F, View, R> ChooseView<R> for F
where
    F: Fn() -> View + Send + 'static,
    View: Render<R> + Send,
    R: Renderer + 'static,
{
    type Output = View;

    async fn choose(self) -> Self::Output {
        self()
    }
}

impl<T, R> ChooseView<R> for Lazy<T>
where
    T: LazyRoute<R>,
    R: Renderer + 'static,
{
    type Output = AnyView<R>;

    async fn choose(self) -> Self::Output {
        T::data().view().await
    }
}

pub trait LazyRoute<R>: Send + 'static
where
    R: Renderer,
{
    fn data() -> Self;

    fn view(self) -> impl Future<Output = AnyView<R>>;
}

#[derive(Debug, Clone)]
pub struct Lazy<T> {
    ty: PhantomData<T>,
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
    type Output = ();

    async fn choose(self) -> Self::Output {}
}

impl<A, B, Rndr> ChooseView<Rndr> for Either<A, B>
where
    A: ChooseView<Rndr>,
    B: ChooseView<Rndr>,
    Rndr: Renderer + 'static,
{
    type Output = Either<A::Output, B::Output>;

    async fn choose(self) -> Self::Output {
        match self {
            Either::Left(f) => Either::Left(f.choose().await),
            Either::Right(f) => Either::Right(f.choose().await),
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
            type Output = $either<$($ty::Output,)*>;

            async fn choose(self ) -> Self::Output {
                match self {
                    $($either::$ty(f) => $either::$ty(f.choose().await),)*
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
