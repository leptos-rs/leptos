#![no_std]

use core::{
    fmt::Display,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use pin_project_lite::pin_project;

#[derive(Debug, Clone, Copy)]
pub enum Either<A, B> {
    Left(A),
    Right(B),
}

impl<Item, A, B> Iterator for Either<A, B>
where
    A: Iterator<Item = Item>,
    B: Iterator<Item = Item>,
{
    type Item = Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Either::Left(i) => i.next(),
            Either::Right(i) => i.next(),
        }
    }
}

pin_project! {
    #[project = EitherFutureProj]
    pub enum EitherFuture<A, B> {
        Left { #[pin] inner: A },
        Right { #[pin] inner: B },
    }
}

impl<A, B> Future for EitherFuture<A, B>
where
    A: Future,
    B: Future,
{
    type Output = Either<A::Output, B::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this {
            EitherFutureProj::Left { inner } => match inner.poll(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(inner) => Poll::Ready(Either::Left(inner)),
            },
            EitherFutureProj::Right { inner } => match inner.poll(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(inner) => Poll::Ready(Either::Right(inner)),
            },
        }
    }
}

macro_rules! tuples {
    ($name:ident + $fut_name:ident + $fut_proj:ident => $($ty:ident),*) => {
        #[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
        pub enum $name<$($ty,)*> {
            $($ty ($ty),)*
        }

        impl<$($ty,)*> Display for $name<$($ty,)*>
        where
            $($ty: Display,)*
        {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                match self {
                    $($name::$ty(this) => this.fmt(f),)*
                }
            }
        }

        impl<Item, $($ty,)*> Iterator for $name<$($ty,)*>
        where
            $($ty: Iterator<Item = Item>,)*
        {
            type Item = Item;

            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    $($name::$ty(i) => i.next(),)*
                }
            }
        }

        pin_project! {
            #[project = $fut_proj]
            pub enum $fut_name<$($ty,)*> {
                $($ty { #[pin] inner: $ty },)*
            }
        }

        impl<$($ty,)*> Future for $fut_name<$($ty,)*>
        where
            $($ty: Future,)*
        {
            type Output = $name<$($ty::Output,)*>;

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let this = self.project();
                match this {
                    $($fut_proj::$ty { inner } => match inner.poll(cx) {
                        Poll::Pending => Poll::Pending,
                        Poll::Ready(inner) => Poll::Ready($name::$ty(inner)),
                    },)*
                }
            }
        }
    }
}

tuples!(EitherOf3 + EitherOf3Future + EitherOf3FutureProj => A, B, C);
tuples!(EitherOf4 + EitherOf4Future + EitherOf4FutureProj => A, B, C, D);
tuples!(EitherOf5 + EitherOf5Future + EitherOf5FutureProj => A, B, C, D, E);
tuples!(EitherOf6 + EitherOf6Future + EitherOf6FutureProj => A, B, C, D, E, F);
tuples!(EitherOf7 + EitherOf7Future + EitherOf7FutureProj => A, B, C, D, E, F, G);
tuples!(EitherOf8 + EitherOf8Future + EitherOf8FutureProj => A, B, C, D, E, F, G, H);
tuples!(EitherOf9 + EitherOf9Future + EitherOf9FutureProj => A, B, C, D, E, F, G, H, I);
tuples!(EitherOf10 + EitherOf10Future + EitherOf10FutureProj => A, B, C, D, E, F, G, H, I, J);
tuples!(EitherOf11 + EitherOf11Future + EitherOf11FutureProj => A, B, C, D, E, F, G, H, I, J, K);
tuples!(EitherOf12 + EitherOf12Future + EitherOf12FutureProj => A, B, C, D, E, F, G, H, I, J, K, L);
tuples!(EitherOf13 + EitherOf13Future + EitherOf13FutureProj => A, B, C, D, E, F, G, H, I, J, K, L, M);
tuples!(EitherOf14 + EitherOf14Future + EitherOf14FutureProj => A, B, C, D, E, F, G, H, I, J, K, L, M, N);
tuples!(EitherOf15 + EitherOf15Future + EitherOf15FutureProj => A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
tuples!(EitherOf16 + EitherOf16Future + EitherOf16FutureProj => A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
