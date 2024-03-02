#![no_std]

use core::fmt::Display;

#[derive(Debug)]
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

macro_rules! tuples {
    ($name:ident => $($ty:ident),*) => {
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

        /*impl<$($ty,)*> Iterator for $name<$($ty,)*>
        where
            $($ty: Iterator,)*
        {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                match self {
                    $($name::$ty(this) => this.fmt(f),)*
                }
            }
        }*/
    }
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
