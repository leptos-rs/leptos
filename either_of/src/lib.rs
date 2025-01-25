#![cfg_attr(feature = "no_std", no_std)]
#![forbid(unsafe_code)]

//! Utilities for working with enumerated types that contain one of `2..n` other types.

use core::{
    cmp::Ordering,
    fmt::Display,
    future::Future,
    iter::{Product, Sum},
    pin::Pin,
    task::{Context, Poll},
};
use paste::paste;
use pin_project_lite::pin_project;
#[cfg(not(feature = "no_std"))]
use std::error::Error; // TODO: replace with core::error::Error once MSRV is >= 1.81.0

macro_rules! tuples {
    ($name:ident + $fut_name:ident + $fut_proj:ident {
        $($ty:ident => ($($rest_variant:ident),*) + <$($mapped_ty:ident),+>),+$(,)?
    }) => {
        tuples!($name + $fut_name + $fut_proj {
            $($ty($ty) => ($($rest_variant),*) + <$($mapped_ty),+>),+
        });
    };
    ($name:ident + $fut_name:ident + $fut_proj:ident {
        $($variant:ident($ty:ident) => ($($rest_variant:ident),*) + <$($mapped_ty:ident),+>),+$(,)?
    }) => {
        #[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
        pub enum $name<$($ty),+> {
            $($variant ($ty),)+
        }

        impl<$($ty),+> $name<$($ty),+> {
            paste! {
                #[allow(clippy::too_many_arguments)]
                pub fn map<$([<F $ty>]),+, $([<$ty 1>]),+>(self, $([<$variant:lower>]: [<F $ty>]),+) -> $name<$([<$ty 1>]),+>
                where
                    $([<F $ty>]: FnOnce($ty) -> [<$ty 1>],)+
                {
                    match self {
                        $($name::$variant(inner) => $name::$variant([<$variant:lower>](inner)),)+
                    }
                }

                $(
                    pub fn [<map_ $variant:lower>]<Fun, [<$ty 1>]>(self, f: Fun) -> $name<$($mapped_ty),+>
                    where
                        Fun: FnOnce($ty) -> [<$ty 1>],
                    {
                        match self {
                            $name::$variant(inner) => $name::$variant(f(inner)),
                            $($name::$rest_variant(inner) => $name::$rest_variant(inner),)*
                        }
                    }

                    pub fn [<inspect_ $variant:lower>]<Fun, [<$ty 1>]>(self, f: Fun) -> Self
                    where
                        Fun: FnOnce(&$ty),
                    {
                        if let $name::$variant(inner) = &self {
                            f(inner);
                        }
                        self
                    }

                    pub fn [<is_ $variant:lower>](&self) -> bool {
                        matches!(self, $name::$variant(_))
                    }

                    pub fn [<as_ $variant:lower>](&self) -> Option<&$ty> {
                        match self {
                            $name::$variant(inner) => Some(inner),
                            _ => None,
                        }
                    }

                    pub fn [<as_ $variant:lower _mut>](&mut self) -> Option<&mut $ty> {
                        match self {
                            $name::$variant(inner) => Some(inner),
                            _ => None,
                        }
                    }

                    pub fn [<unwrap_ $variant:lower>](self) -> $ty {
                        match self {
                            $name::$variant(inner) => inner,
                            _ => panic!(concat!(
                                "called `unwrap_", stringify!([<$variant:lower>]), "()` on a non-`", stringify!($variant), "` variant of `", stringify!($name), "`"
                            )),
                        }
                    }

                    pub fn [<into_ $variant:lower>](self) -> Result<$ty, Self> {
                        match self {
                            $name::$variant(inner) => Ok(inner),
                            _ => Err(self),
                        }
                    }
                )+
            }
        }

        impl<$($ty),+> Display for $name<$($ty),+>
        where
            $($ty: Display,)+
        {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                match self {
                    $($name::$variant(this) => this.fmt(f),)+
                }
            }
        }

        #[cfg(not(feature = "no_std"))]
        impl<$($ty),+> Error for $name<$($ty),+>
        where
            $($ty: Error,)+
        {
            fn source(&self) -> Option<&(dyn Error + 'static)> {
                match self {
                    $($name::$variant(this) => this.source(),)+
                }
            }
        }

        impl<Item, $($ty),+> Iterator for $name<$($ty),+>
        where
            $($ty: Iterator<Item = Item>,)+
        {
            type Item = Item;

            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    $($name::$variant(i) => i.next(),)+
                }
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                match self {
                    $($name::$variant(i) => i.size_hint(),)+
                }
            }

            fn count(self) -> usize
            where
                Self: Sized,
            {
                match self {
                    $($name::$variant(i) => i.count(),)+
                }
            }

            fn last(self) -> Option<Self::Item>
            where
                Self: Sized,
            {
                match self {
                    $($name::$variant(i) => i.last(),)+
                }
            }

            fn nth(&mut self, n: usize) -> Option<Self::Item> {
                match self {
                    $($name::$variant(i) => i.nth(n),)+
                }
            }

            fn for_each<Fun>(self, f: Fun)
            where
                Self: Sized,
                Fun: FnMut(Self::Item),
            {
                match self {
                    $($name::$variant(i) => i.for_each(f),)+
                }
            }

            fn collect<Col: FromIterator<Self::Item>>(self) -> Col
            where
                Self: Sized,
            {
                match self {
                    $($name::$variant(i) => i.collect(),)+
                }
            }

            fn partition<Col, Fun>(self, f: Fun) -> (Col, Col)
            where
                Self: Sized,
                Col: Default + Extend<Self::Item>,
                Fun: FnMut(&Self::Item) -> bool,
            {
                match self {
                    $($name::$variant(i) => i.partition(f),)+
                }
            }

            fn fold<Acc, Fun>(self, init: Acc, f: Fun) -> Acc
            where
                Self: Sized,
                Fun: FnMut(Acc, Self::Item) -> Acc,
            {
                match self {
                    $($name::$variant(i) => i.fold(init, f),)+
                }
            }

            fn reduce<Fun>(self, f: Fun) -> Option<Self::Item>
            where
                Self: Sized,
                Fun: FnMut(Self::Item, Self::Item) -> Self::Item,
            {
                match self {
                    $($name::$variant(i) => i.reduce(f),)+
                }
            }

            fn all<Fun>(&mut self, f: Fun) -> bool
            where
                Self: Sized,
                Fun: FnMut(Self::Item) -> bool,
            {
                match self {
                    $($name::$variant(i) => i.all(f),)+
                }
            }

            fn any<Fun>(&mut self, f: Fun) -> bool
            where
                Self: Sized,
                Fun: FnMut(Self::Item) -> bool,
            {
                match self {
                    $($name::$variant(i) => i.any(f),)+
                }
            }

            fn find<Pre>(&mut self, predicate: Pre) -> Option<Self::Item>
            where
                Self: Sized,
                Pre: FnMut(&Self::Item) -> bool,
            {
                match self {
                    $($name::$variant(i) => i.find(predicate),)+
                }
            }

            fn find_map<Out, Fun>(&mut self, f: Fun) -> Option<Out>
            where
                Self: Sized,
                Fun: FnMut(Self::Item) -> Option<Out>,
            {
                match self {
                    $($name::$variant(i) => i.find_map(f),)+
                }
            }

            fn position<Pre>(&mut self, predicate: Pre) -> Option<usize>
            where
                Self: Sized,
                Pre: FnMut(Self::Item) -> bool,
            {
                match self {
                    $($name::$variant(i) => i.position(predicate),)+
                }
            }

            fn max(self) -> Option<Self::Item>
            where
                Self: Sized,
                Self::Item: Ord,
            {
                match self {
                    $($name::$variant(i) => i.max(),)+
                }
            }

            fn min(self) -> Option<Self::Item>
            where
                Self: Sized,
                Self::Item: Ord,
            {
                match self {
                    $($name::$variant(i) => i.min(),)+
                }
            }

            fn max_by_key<Key: Ord, Fun>(self, f: Fun) -> Option<Self::Item>
            where
                Self: Sized,
                Fun: FnMut(&Self::Item) -> Key,
            {
                match self {
                    $($name::$variant(i) => i.max_by_key(f),)+
                }
            }

            fn max_by<Cmp>(self, compare: Cmp) -> Option<Self::Item>
            where
                Self: Sized,
                Cmp: FnMut(&Self::Item, &Self::Item) -> Ordering,
            {
                match self {
                    $($name::$variant(i) => i.max_by(compare),)+
                }
            }

            fn min_by_key<Key: Ord, Fun>(self, f: Fun) -> Option<Self::Item>
            where
                Self: Sized,
                Fun: FnMut(&Self::Item) -> Key,
            {
                match self {
                    $($name::$variant(i) => i.min_by_key(f),)+
                }
            }

            fn min_by<Cmp>(self, compare: Cmp) -> Option<Self::Item>
            where
                Self: Sized,
                Cmp: FnMut(&Self::Item, &Self::Item) -> Ordering,
            {
                match self {
                    $($name::$variant(i) => i.min_by(compare),)+
                }
            }

            fn sum<Out>(self) -> Out
            where
                Self: Sized,
                Out: Sum<Self::Item>,
            {
                match self {
                    $($name::$variant(i) => i.sum(),)+
                }
            }

            fn product<Out>(self) -> Out
            where
                Self: Sized,
                Out: Product<Self::Item>,
            {
                match self {
                    $($name::$variant(i) => i.product(),)+
                }
            }

            fn cmp<Other>(self, other: Other) -> Ordering
            where
                Other: IntoIterator<Item = Self::Item>,
                Self::Item: Ord,
                Self: Sized,
            {
                match self {
                    $($name::$variant(i) => i.cmp(other),)+
                }
            }

            fn partial_cmp<Other>(self, other: Other) -> Option<Ordering>
            where
                Other: IntoIterator,
                Self::Item: PartialOrd<Other::Item>,
                Self: Sized,
            {
                match self {
                    $($name::$variant(i) => i.partial_cmp(other),)+
                }
            }

            // TODO: uncomment once MSRV is >= 1.82.0
            // fn is_sorted(self) -> bool
            // where
            //     Self: Sized,
            //     Self::Item: PartialOrd,
            // {
            //     match self {
            //         $($name::$variant(i) => i.is_sorted(),)+
            //     }
            // }
            //
            // fn is_sorted_by<Cmp>(self, compare: Cmp) -> bool
            // where
            //     Self: Sized,
            //     Cmp: FnMut(&Self::Item, &Self::Item) -> bool,
            // {
            //     match self {
            //         $($name::$variant(i) => i.is_sorted_by(compare),)+
            //     }
            // }
            //
            // fn is_sorted_by_key<Fun, Key>(self, f: Fun) -> bool
            // where
            //     Self: Sized,
            //     Fun: FnMut(Self::Item) -> Key,
            //     Key: PartialOrd,
            // {
            //     match self {
            //         $($name::$variant(i) => i.is_sorted_by_key(f),)+
            //     }
            // }
        }

        impl<Item, $($ty),+> ExactSizeIterator for $name<$($ty),+>
        where
            $($ty: ExactSizeIterator<Item = Item>,)+
        {
            fn len(&self) -> usize {
                match self {
                    $($name::$variant(i) => i.len(),)+
                }
            }
        }

        impl<Item, $($ty),+> DoubleEndedIterator for $name<$($ty),+>
        where
            $($ty: DoubleEndedIterator<Item = Item>,)+
        {
            fn next_back(&mut self) -> Option<Self::Item> {
                match self {
                    $($name::$variant(i) => i.next_back(),)+
                }
            }

            fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
                match self {
                    $($name::$variant(i) => i.nth_back(n),)+
                }
            }

            fn rfind<Pre>(&mut self, predicate: Pre) -> Option<Self::Item>
            where
                Pre: FnMut(&Self::Item) -> bool,
            {
                match self {
                    $($name::$variant(i) => i.rfind(predicate),)+
                }
            }
        }

        pin_project! {
            #[project = $fut_proj]
            pub enum $fut_name<$($ty),+> {
                $($variant { #[pin] inner: $ty },)+
            }
        }

        impl<$($ty),+> Future for $fut_name<$($ty),+>
        where
            $($ty: Future,)+
        {
            type Output = $name<$($ty::Output),+>;

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let this = self.project();
                match this {
                    $($fut_proj::$variant { inner } => match inner.poll(cx) {
                        Poll::Pending => Poll::Pending,
                        Poll::Ready(inner) => Poll::Ready($name::$variant(inner)),
                    },)+
                }
            }
        }
    }
}

tuples!(Either + EitherFuture + EitherFutureProj {
    Left(A) => (Right) + <A1, B>,
    Right(B) => (Left) + <A, B1>,
});

impl<A, B> Either<A, B> {
    pub fn swap(self) -> Either<B, A> {
        match self {
            Either::Left(a) => Either::Right(a),
            Either::Right(b) => Either::Left(b),
        }
    }
}

impl<A, B> From<Result<A, B>> for Either<A, B> {
    fn from(value: Result<A, B>) -> Self {
        match value {
            Ok(left) => Either::Left(left),
            Err(right) => Either::Right(right),
        }
    }
}

pub trait EitherOr {
    type Left;
    type Right;
    fn either_or<FA, A, FB, B>(self, a: FA, b: FB) -> Either<A, B>
    where
        FA: FnOnce(Self::Left) -> A,
        FB: FnOnce(Self::Right) -> B;
}

impl EitherOr for bool {
    type Left = ();
    type Right = ();

    fn either_or<FA, A, FB, B>(self, a: FA, b: FB) -> Either<A, B>
    where
        FA: FnOnce(Self::Left) -> A,
        FB: FnOnce(Self::Right) -> B,
    {
        if self {
            Either::Left(a(()))
        } else {
            Either::Right(b(()))
        }
    }
}

impl<T> EitherOr for Option<T> {
    type Left = T;
    type Right = ();

    fn either_or<FA, A, FB, B>(self, a: FA, b: FB) -> Either<A, B>
    where
        FA: FnOnce(Self::Left) -> A,
        FB: FnOnce(Self::Right) -> B,
    {
        match self {
            Some(t) => Either::Left(a(t)),
            None => Either::Right(b(())),
        }
    }
}

impl<T, E> EitherOr for Result<T, E> {
    type Left = T;
    type Right = E;

    fn either_or<FA, A, FB, B>(self, a: FA, b: FB) -> Either<A, B>
    where
        FA: FnOnce(Self::Left) -> A,
        FB: FnOnce(Self::Right) -> B,
    {
        match self {
            Ok(t) => Either::Left(a(t)),
            Err(err) => Either::Right(b(err)),
        }
    }
}

impl<A, B> EitherOr for Either<A, B> {
    type Left = A;
    type Right = B;

    #[inline]
    fn either_or<FA, A1, FB, B1>(self, a: FA, b: FB) -> Either<A1, B1>
    where
        FA: FnOnce(<Self as EitherOr>::Left) -> A1,
        FB: FnOnce(<Self as EitherOr>::Right) -> B1,
    {
        self.map(a, b)
    }
}

#[test]
fn test_either_or() {
    let right = false.either_or(|_| 'a', |_| 12);
    assert!(matches!(right, Either::Right(12)));

    let left = true.either_or(|_| 'a', |_| 12);
    assert!(matches!(left, Either::Left('a')));

    let left = Some(12).either_or(|a| a, |_| 'a');
    assert!(matches!(left, Either::Left(12)));
    let right = None.either_or(|a: i32| a, |_| 'a');
    assert!(matches!(right, Either::Right('a')));

    let result: Result<_, ()> = Ok(1.2f32);
    let left = result.either_or(|a| a * 2f32, |b| b);
    assert!(matches!(left, Either::Left(2.4f32)));

    let result: Result<i32, _> = Err("12");
    let right = result.either_or(|a| a, |b| b.chars().next());
    assert!(matches!(right, Either::Right(Some('1'))));

    let either = Either::<i32, char>::Left(12);
    let left = either.either_or(|a| a, |b| b);
    assert!(matches!(left, Either::Left(12)));

    let either = Either::<i32, char>::Right('a');
    let right = either.either_or(|a| a, |b| b);
    assert!(matches!(right, Either::Right('a')));
}

tuples!(EitherOf3 + EitherOf3Future + EitherOf3FutureProj {
    A => (B, C) + <A1, B, C>,
    B => (A, C) + <A, B1, C>,
    C => (A, B) + <A, B, C1>,
});
tuples!(EitherOf4 + EitherOf4Future + EitherOf4FutureProj {
    A => (B, C, D) + <A1, B, C, D>,
    B => (A, C, D) + <A, B1, C, D>,
    C => (A, B, D) + <A, B, C1, D>,
    D => (A, B, C) + <A, B, C, D1>,
});
tuples!(EitherOf5 + EitherOf5Future + EitherOf5FutureProj {
    A => (B, C, D, E) + <A1, B, C, D, E>,
    B => (A, C, D, E) + <A, B1, C, D, E>,
    C => (A, B, D, E) + <A, B, C1, D, E>,
    D => (A, B, C, E) + <A, B, C, D1, E>,
    E => (A, B, C, D) + <A, B, C, D, E1>,
});
tuples!(EitherOf6 + EitherOf6Future + EitherOf6FutureProj {
    A => (B, C, D, E, F) + <A1, B, C, D, E, F>,
    B => (A, C, D, E, F) + <A, B1, C, D, E, F>,
    C => (A, B, D, E, F) + <A, B, C1, D, E, F>,
    D => (A, B, C, E, F) + <A, B, C, D1, E, F>,
    E => (A, B, C, D, F) + <A, B, C, D, E1, F>,
    F => (A, B, C, D, E) + <A, B, C, D, E, F1>,
});
tuples!(EitherOf7 + EitherOf7Future + EitherOf7FutureProj {
    A => (B, C, D, E, F, G) + <A1, B, C, D, E, F, G>,
    B => (A, C, D, E, F, G) + <A, B1, C, D, E, F, G>,
    C => (A, B, D, E, F, G) + <A, B, C1, D, E, F, G>,
    D => (A, B, C, E, F, G) + <A, B, C, D1, E, F, G>,
    E => (A, B, C, D, F, G) + <A, B, C, D, E1, F, G>,
    F => (A, B, C, D, E, G) + <A, B, C, D, E, F1, G>,
    G => (A, B, C, D, E, F) + <A, B, C, D, E, F, G1>,
});
tuples!(EitherOf8 + EitherOf8Future + EitherOf8FutureProj {
    A => (B, C, D, E, F, G, H) + <A1, B, C, D, E, F, G, H>,
    B => (A, C, D, E, F, G, H) + <A, B1, C, D, E, F, G, H>,
    C => (A, B, D, E, F, G, H) + <A, B, C1, D, E, F, G, H>,
    D => (A, B, C, E, F, G, H) + <A, B, C, D1, E, F, G, H>,
    E => (A, B, C, D, F, G, H) + <A, B, C, D, E1, F, G, H>,
    F => (A, B, C, D, E, G, H) + <A, B, C, D, E, F1, G, H>,
    G => (A, B, C, D, E, F, H) + <A, B, C, D, E, F, G1, H>,
    H => (A, B, C, D, E, F, G) + <A, B, C, D, E, F, G, H1>,
});
tuples!(EitherOf9 + EitherOf9Future + EitherOf9FutureProj {
    A => (B, C, D, E, F, G, H, I) + <A1, B, C, D, E, F, G, H, I>,
    B => (A, C, D, E, F, G, H, I) + <A, B1, C, D, E, F, G, H, I>,
    C => (A, B, D, E, F, G, H, I) + <A, B, C1, D, E, F, G, H, I>,
    D => (A, B, C, E, F, G, H, I) + <A, B, C, D1, E, F, G, H, I>,
    E => (A, B, C, D, F, G, H, I) + <A, B, C, D, E1, F, G, H, I>,
    F => (A, B, C, D, E, G, H, I) + <A, B, C, D, E, F1, G, H, I>,
    G => (A, B, C, D, E, F, H, I) + <A, B, C, D, E, F, G1, H, I>,
    H => (A, B, C, D, E, F, G, I) + <A, B, C, D, E, F, G, H1, I>,
    I => (A, B, C, D, E, F, G, H) + <A, B, C, D, E, F, G, H, I1>,
});
tuples!(EitherOf10 + EitherOf10Future + EitherOf10FutureProj {
    A => (B, C, D, E, F, G, H, I, J) + <A1, B, C, D, E, F, G, H, I, J>,
    B => (A, C, D, E, F, G, H, I, J) + <A, B1, C, D, E, F, G, H, I, J>,
    C => (A, B, D, E, F, G, H, I, J) + <A, B, C1, D, E, F, G, H, I, J>,
    D => (A, B, C, E, F, G, H, I, J) + <A, B, C, D1, E, F, G, H, I, J>,
    E => (A, B, C, D, F, G, H, I, J) + <A, B, C, D, E1, F, G, H, I, J>,
    F => (A, B, C, D, E, G, H, I, J) + <A, B, C, D, E, F1, G, H, I, J>,
    G => (A, B, C, D, E, F, H, I, J) + <A, B, C, D, E, F, G1, H, I, J>,
    H => (A, B, C, D, E, F, G, I, J) + <A, B, C, D, E, F, G, H1, I, J>,
    I => (A, B, C, D, E, F, G, H, J) + <A, B, C, D, E, F, G, H, I1, J>,
    J => (A, B, C, D, E, F, G, H, I) + <A, B, C, D, E, F, G, H, I, J1>,
});
tuples!(EitherOf11 + EitherOf11Future + EitherOf11FutureProj {
    A => (B, C, D, E, F, G, H, I, J, K) + <A1, B, C, D, E, F, G, H, I, J, K>,
    B => (A, C, D, E, F, G, H, I, J, K) + <A, B1, C, D, E, F, G, H, I, J, K>,
    C => (A, B, D, E, F, G, H, I, J, K) + <A, B, C1, D, E, F, G, H, I, J, K>,
    D => (A, B, C, E, F, G, H, I, J, K) + <A, B, C, D1, E, F, G, H, I, J, K>,
    E => (A, B, C, D, F, G, H, I, J, K) + <A, B, C, D, E1, F, G, H, I, J, K>,
    F => (A, B, C, D, E, G, H, I, J, K) + <A, B, C, D, E, F1, G, H, I, J, K>,
    G => (A, B, C, D, E, F, H, I, J, K) + <A, B, C, D, E, F, G1, H, I, J, K>,
    H => (A, B, C, D, E, F, G, I, J, K) + <A, B, C, D, E, F, G, H1, I, J, K>,
    I => (A, B, C, D, E, F, G, H, J, K) + <A, B, C, D, E, F, G, H, I1, J, K>,
    J => (A, B, C, D, E, F, G, H, I, K) + <A, B, C, D, E, F, G, H, I, J1, K>,
    K => (A, B, C, D, E, F, G, H, I, J) + <A, B, C, D, E, F, G, H, I, J, K1>,
});
tuples!(EitherOf12 + EitherOf12Future + EitherOf12FutureProj {
    A => (B, C, D, E, F, G, H, I, J, K, L) + <A1, B, C, D, E, F, G, H, I, J, K, L>,
    B => (A, C, D, E, F, G, H, I, J, K, L) + <A, B1, C, D, E, F, G, H, I, J, K, L>,
    C => (A, B, D, E, F, G, H, I, J, K, L) + <A, B, C1, D, E, F, G, H, I, J, K, L>,
    D => (A, B, C, E, F, G, H, I, J, K, L) + <A, B, C, D1, E, F, G, H, I, J, K, L>,
    E => (A, B, C, D, F, G, H, I, J, K, L) + <A, B, C, D, E1, F, G, H, I, J, K, L>,
    F => (A, B, C, D, E, G, H, I, J, K, L) + <A, B, C, D, E, F1, G, H, I, J, K, L>,
    G => (A, B, C, D, E, F, H, I, J, K, L) + <A, B, C, D, E, F, G1, H, I, J, K, L>,
    H => (A, B, C, D, E, F, G, I, J, K, L) + <A, B, C, D, E, F, G, H1, I, J, K, L>,
    I => (A, B, C, D, E, F, G, H, J, K, L) + <A, B, C, D, E, F, G, H, I1, J, K, L>,
    J => (A, B, C, D, E, F, G, H, I, K, L) + <A, B, C, D, E, F, G, H, I, J1, K, L>,
    K => (A, B, C, D, E, F, G, H, I, J, L) + <A, B, C, D, E, F, G, H, I, J, K1, L>,
    L => (A, B, C, D, E, F, G, H, I, J, K) + <A, B, C, D, E, F, G, H, I, J, K, L1>,
});
tuples!(EitherOf13 + EitherOf13Future + EitherOf13FutureProj {
    A => (B, C, D, E, F, G, H, I, J, K, L, M) + <A1, B, C, D, E, F, G, H, I, J, K, L, M>,
    B => (A, C, D, E, F, G, H, I, J, K, L, M) + <A, B1, C, D, E, F, G, H, I, J, K, L, M>,
    C => (A, B, D, E, F, G, H, I, J, K, L, M) + <A, B, C1, D, E, F, G, H, I, J, K, L, M>,
    D => (A, B, C, E, F, G, H, I, J, K, L, M) + <A, B, C, D1, E, F, G, H, I, J, K, L, M>,
    E => (A, B, C, D, F, G, H, I, J, K, L, M) + <A, B, C, D, E1, F, G, H, I, J, K, L, M>,
    F => (A, B, C, D, E, G, H, I, J, K, L, M) + <A, B, C, D, E, F1, G, H, I, J, K, L, M>,
    G => (A, B, C, D, E, F, H, I, J, K, L, M) + <A, B, C, D, E, F, G1, H, I, J, K, L, M>,
    H => (A, B, C, D, E, F, G, I, J, K, L, M) + <A, B, C, D, E, F, G, H1, I, J, K, L, M>,
    I => (A, B, C, D, E, F, G, H, J, K, L, M) + <A, B, C, D, E, F, G, H, I1, J, K, L, M>,
    J => (A, B, C, D, E, F, G, H, I, K, L, M) + <A, B, C, D, E, F, G, H, I, J1, K, L, M>,
    K => (A, B, C, D, E, F, G, H, I, J, L, M) + <A, B, C, D, E, F, G, H, I, J, K1, L, M>,
    L => (A, B, C, D, E, F, G, H, I, J, K, M) + <A, B, C, D, E, F, G, H, I, J, K, L1, M>,
    M => (A, B, C, D, E, F, G, H, I, J, K, L) + <A, B, C, D, E, F, G, H, I, J, K, L, M1>,
});
tuples!(EitherOf14 + EitherOf14Future + EitherOf14FutureProj {
    A => (B, C, D, E, F, G, H, I, J, K, L, M, N) + <A1, B, C, D, E, F, G, H, I, J, K, L, M, N>,
    B => (A, C, D, E, F, G, H, I, J, K, L, M, N) + <A, B1, C, D, E, F, G, H, I, J, K, L, M, N>,
    C => (A, B, D, E, F, G, H, I, J, K, L, M, N) + <A, B, C1, D, E, F, G, H, I, J, K, L, M, N>,
    D => (A, B, C, E, F, G, H, I, J, K, L, M, N) + <A, B, C, D1, E, F, G, H, I, J, K, L, M, N>,
    E => (A, B, C, D, F, G, H, I, J, K, L, M, N) + <A, B, C, D, E1, F, G, H, I, J, K, L, M, N>,
    F => (A, B, C, D, E, G, H, I, J, K, L, M, N) + <A, B, C, D, E, F1, G, H, I, J, K, L, M, N>,
    G => (A, B, C, D, E, F, H, I, J, K, L, M, N) + <A, B, C, D, E, F, G1, H, I, J, K, L, M, N>,
    H => (A, B, C, D, E, F, G, I, J, K, L, M, N) + <A, B, C, D, E, F, G, H1, I, J, K, L, M, N>,
    I => (A, B, C, D, E, F, G, H, J, K, L, M, N) + <A, B, C, D, E, F, G, H, I1, J, K, L, M, N>,
    J => (A, B, C, D, E, F, G, H, I, K, L, M, N) + <A, B, C, D, E, F, G, H, I, J1, K, L, M, N>,
    K => (A, B, C, D, E, F, G, H, I, J, L, M, N) + <A, B, C, D, E, F, G, H, I, J, K1, L, M, N>,
    L => (A, B, C, D, E, F, G, H, I, J, K, M, N) + <A, B, C, D, E, F, G, H, I, J, K, L1, M, N>,
    M => (A, B, C, D, E, F, G, H, I, J, K, L, N) + <A, B, C, D, E, F, G, H, I, J, K, L, M1, N>,
    N => (A, B, C, D, E, F, G, H, I, J, K, L, M) + <A, B, C, D, E, F, G, H, I, J, K, L, M, N1>,
});
tuples!(EitherOf15 + EitherOf15Future + EitherOf15FutureProj {
    A => (B, C, D, E, F, G, H, I, J, K, L, M, N, O) + <A1, B, C, D, E, F, G, H, I, J, K, L, M, N, O>,
    B => (A, C, D, E, F, G, H, I, J, K, L, M, N, O) + <A, B1, C, D, E, F, G, H, I, J, K, L, M, N, O>,
    C => (A, B, D, E, F, G, H, I, J, K, L, M, N, O) + <A, B, C1, D, E, F, G, H, I, J, K, L, M, N, O>,
    D => (A, B, C, E, F, G, H, I, J, K, L, M, N, O) + <A, B, C, D1, E, F, G, H, I, J, K, L, M, N, O>,
    E => (A, B, C, D, F, G, H, I, J, K, L, M, N, O) + <A, B, C, D, E1, F, G, H, I, J, K, L, M, N, O>,
    F => (A, B, C, D, E, G, H, I, J, K, L, M, N, O) + <A, B, C, D, E, F1, G, H, I, J, K, L, M, N, O>,
    G => (A, B, C, D, E, F, H, I, J, K, L, M, N, O) + <A, B, C, D, E, F, G1, H, I, J, K, L, M, N, O>,
    H => (A, B, C, D, E, F, G, I, J, K, L, M, N, O) + <A, B, C, D, E, F, G, H1, I, J, K, L, M, N, O>,
    I => (A, B, C, D, E, F, G, H, J, K, L, M, N, O) + <A, B, C, D, E, F, G, H, I1, J, K, L, M, N, O>,
    J => (A, B, C, D, E, F, G, H, I, K, L, M, N, O) + <A, B, C, D, E, F, G, H, I, J1, K, L, M, N, O>,
    K => (A, B, C, D, E, F, G, H, I, J, L, M, N, O) + <A, B, C, D, E, F, G, H, I, J, K1, L, M, N, O>,
    L => (A, B, C, D, E, F, G, H, I, J, K, M, N, O) + <A, B, C, D, E, F, G, H, I, J, K, L1, M, N, O>,
    M => (A, B, C, D, E, F, G, H, I, J, K, L, N, O) + <A, B, C, D, E, F, G, H, I, J, K, L, M1, N, O>,
    N => (A, B, C, D, E, F, G, H, I, J, K, L, M, O) + <A, B, C, D, E, F, G, H, I, J, K, L, M, N1, O>,
    O => (A, B, C, D, E, F, G, H, I, J, K, L, M, N) + <A, B, C, D, E, F, G, H, I, J, K, L, M, N, O1>,
});
tuples!(EitherOf16 + EitherOf16Future + EitherOf16FutureProj {
    A => (B, C, D, E, F, G, H, I, J, K, L, M, N, O, P) + <A1, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P>,
    B => (A, C, D, E, F, G, H, I, J, K, L, M, N, O, P) + <A, B1, C, D, E, F, G, H, I, J, K, L, M, N, O, P>,
    C => (A, B, D, E, F, G, H, I, J, K, L, M, N, O, P) + <A, B, C1, D, E, F, G, H, I, J, K, L, M, N, O, P>,
    D => (A, B, C, E, F, G, H, I, J, K, L, M, N, O, P) + <A, B, C, D1, E, F, G, H, I, J, K, L, M, N, O, P>,
    E => (A, B, C, D, F, G, H, I, J, K, L, M, N, O, P) + <A, B, C, D, E1, F, G, H, I, J, K, L, M, N, O, P>,
    F => (A, B, C, D, E, G, H, I, J, K, L, M, N, O, P) + <A, B, C, D, E, F1, G, H, I, J, K, L, M, N, O, P>,
    G => (A, B, C, D, E, F, H, I, J, K, L, M, N, O, P) + <A, B, C, D, E, F, G1, H, I, J, K, L, M, N, O, P>,
    H => (A, B, C, D, E, F, G, I, J, K, L, M, N, O, P) + <A, B, C, D, E, F, G, H1, I, J, K, L, M, N, O, P>,
    I => (A, B, C, D, E, F, G, H, J, K, L, M, N, O, P) + <A, B, C, D, E, F, G, H, I1, J, K, L, M, N, O, P>,
    J => (A, B, C, D, E, F, G, H, I, K, L, M, N, O, P) + <A, B, C, D, E, F, G, H, I, J1, K, L, M, N, O, P>,
    K => (A, B, C, D, E, F, G, H, I, J, L, M, N, O, P) + <A, B, C, D, E, F, G, H, I, J, K1, L, M, N, O, P>,
    L => (A, B, C, D, E, F, G, H, I, J, K, M, N, O, P) + <A, B, C, D, E, F, G, H, I, J, K, L1, M, N, O, P>,
    M => (A, B, C, D, E, F, G, H, I, J, K, L, N, O, P) + <A, B, C, D, E, F, G, H, I, J, K, L, M1, N, O, P>,
    N => (A, B, C, D, E, F, G, H, I, J, K, L, M, O, P) + <A, B, C, D, E, F, G, H, I, J, K, L, M, N1, O, P>,
    O => (A, B, C, D, E, F, G, H, I, J, K, L, M, N, P) + <A, B, C, D, E, F, G, H, I, J, K, L, M, N, O1, P>,
    P => (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O) + <A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P1>,
});

/// Matches over the first expression and returns an either ([`Either`], [`EitherOf3`], ... [`EitherOf8`])
/// composed of the values returned by the match arms.
///
/// The pattern syntax is exactly the same as found in a match arm.
///
/// # Examples
///
/// ```
/// # use either_of::*;
/// let either2 = either!(Some("hello"),
///     Some(s) => s.len(),
///     None => 0.0,
/// );
/// assert!(matches!(either2, Either::<usize, f64>::Left(5)));
///
/// let either3 = either!(Some("admin"),
///     Some("admin") => "hello admin",
///     Some(_) => 'x',
///     _ => 0,
/// );
/// assert!(matches!(either3, EitherOf3::<&str, char, i32>::A("hello admin")));
/// ```
#[macro_export]
macro_rules! either {
    ($match:expr, $left_pattern:pat => $left_expression:expr, $right_pattern:pat => $right_expression:expr,) => {
        match $match {
            $left_pattern => $crate::Either::Left($left_expression),
            $right_pattern => $crate::Either::Right($right_expression),
        }
    };
    ($match:expr, $a_pattern:pat => $a_expression:expr, $b_pattern:pat => $b_expression:expr, $c_pattern:pat => $c_expression:expr,) => {
        match $match {
            $a_pattern => $crate::EitherOf3::A($a_expression),
            $b_pattern => $crate::EitherOf3::B($b_expression),
            $c_pattern => $crate::EitherOf3::C($c_expression),
        }
    };
    ($match:expr, $a_pattern:pat => $a_expression:expr, $b_pattern:pat => $b_expression:expr, $c_pattern:pat => $c_expression:expr, $d_pattern:pat => $d_expression:expr,) => {
        match $match {
            $a_pattern => $crate::EitherOf4::A($a_expression),
            $b_pattern => $crate::EitherOf4::B($b_expression),
            $c_pattern => $crate::EitherOf4::C($c_expression),
            $d_pattern => $crate::EitherOf4::D($d_expression),
        }
    };
    ($match:expr, $a_pattern:pat => $a_expression:expr, $b_pattern:pat => $b_expression:expr, $c_pattern:pat => $c_expression:expr, $d_pattern:pat => $d_expression:expr, $e_pattern:pat => $e_expression:expr,) => {
        match $match {
            $a_pattern => $crate::EitherOf5::A($a_expression),
            $b_pattern => $crate::EitherOf5::B($b_expression),
            $c_pattern => $crate::EitherOf5::C($c_expression),
            $d_pattern => $crate::EitherOf5::D($d_expression),
            $e_pattern => $crate::EitherOf5::E($e_expression),
        }
    };
    ($match:expr, $a_pattern:pat => $a_expression:expr, $b_pattern:pat => $b_expression:expr, $c_pattern:pat => $c_expression:expr, $d_pattern:pat => $d_expression:expr, $e_pattern:pat => $e_expression:expr, $f_pattern:pat => $f_expression:expr,) => {
        match $match {
            $a_pattern => $crate::EitherOf6::A($a_expression),
            $b_pattern => $crate::EitherOf6::B($b_expression),
            $c_pattern => $crate::EitherOf6::C($c_expression),
            $d_pattern => $crate::EitherOf6::D($d_expression),
            $e_pattern => $crate::EitherOf6::E($e_expression),
            $f_pattern => $crate::EitherOf6::F($f_expression),
        }
    };
    ($match:expr, $a_pattern:pat => $a_expression:expr, $b_pattern:pat => $b_expression:expr, $c_pattern:pat => $c_expression:expr, $d_pattern:pat => $d_expression:expr, $e_pattern:pat => $e_expression:expr, $f_pattern:pat => $f_expression:expr, $g_pattern:pat => $g_expression:expr,) => {
        match $match {
            $a_pattern => $crate::EitherOf7::A($a_expression),
            $b_pattern => $crate::EitherOf7::B($b_expression),
            $c_pattern => $crate::EitherOf7::C($c_expression),
            $d_pattern => $crate::EitherOf7::D($d_expression),
            $e_pattern => $crate::EitherOf7::E($e_expression),
            $f_pattern => $crate::EitherOf7::F($f_expression),
            $g_pattern => $crate::EitherOf7::G($g_expression),
        }
    };
    ($match:expr, $a_pattern:pat => $a_expression:expr, $b_pattern:pat => $b_expression:expr, $c_pattern:pat => $c_expression:expr, $d_pattern:pat => $d_expression:expr, $e_pattern:pat => $e_expression:expr, $f_pattern:pat => $f_expression:expr, $g_pattern:pat => $g_expression:expr, $h_pattern:pat => $h_expression:expr,) => {
        match $match {
            $a_pattern => $crate::EitherOf8::A($a_expression),
            $b_pattern => $crate::EitherOf8::B($b_expression),
            $c_pattern => $crate::EitherOf8::C($c_expression),
            $d_pattern => $crate::EitherOf8::D($d_expression),
            $e_pattern => $crate::EitherOf8::E($e_expression),
            $f_pattern => $crate::EitherOf8::F($f_expression),
            $g_pattern => $crate::EitherOf8::G($g_expression),
            $h_pattern => $crate::EitherOf8::H($h_expression),
        }
    }; // if you need more eithers feel free to open a PR ;-)
}

#[cfg(test)]
mod tests {
    use super::*;

    // compile time test
    #[test]
    fn either_macro() {
        let _: Either<&str, f64> = either!(12,
            12 => "12",
            _ => 0.0,
        );
        let _: EitherOf3<&str, f64, i32> = either!(12,
            12 => "12",
            13 => 0.0,
            _ => 12,
        );
        let _: EitherOf4<&str, f64, char, i32> = either!(12,
            12 => "12",
            13 => 0.0,
            14 => ' ',
            _ => 12,
        );
        let _: EitherOf5<&str, f64, char, f32, i32> = either!(12,
            12 => "12",
            13 => 0.0,
            14 => ' ',
            15 => 0.0f32,
            _ => 12,
        );
        let _: EitherOf6<&str, f64, char, f32, u8, i32> = either!(12,
            12 => "12",
            13 => 0.0,
            14 => ' ',
            15 => 0.0f32,
            16 => 24u8,
            _ => 12,
        );
        let _: EitherOf7<&str, f64, char, f32, u8, i8, i32> = either!(12,
            12 => "12",
            13 => 0.0,
            14 => ' ',
            15 => 0.0f32,
            16 => 24u8,
            17 => 2i8,
            _ => 12,
        );
        let _: EitherOf8<&str, f64, char, f32, u8, i8, u32, i32> = either!(12,
            12 => "12",
            13 => 0.0,
            14 => ' ',
            15 => 0.0f32,
            16 => 24u8,
            17 => 2i8,
            18 => 42u32,
            _ => 12,
        );
    }

    #[test]
    #[should_panic]
    fn unwrap_wrong_either() {
        Either::<i32, &str>::Left(0).unwrap_right();
    }
}
