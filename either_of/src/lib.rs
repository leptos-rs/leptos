#![no_std]
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

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Either::Left(i) => i.size_hint(),
            Either::Right(i) => i.size_hint(),
        }
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        match self {
            Either::Left(i) => i.count(),
            Either::Right(i) => i.count(),
        }
    }

    fn last(self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        match self {
            Either::Left(i) => i.last(),
            Either::Right(i) => i.last(),
        }
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        match self {
            Either::Left(i) => i.nth(n),
            Either::Right(i) => i.nth(n),
        }
    }

    fn for_each<Fun>(self, f: Fun)
    where
        Self: Sized,
        Fun: FnMut(Self::Item),
    {
        match self {
            Either::Left(i) => i.for_each(f),
            Either::Right(i) => i.for_each(f),
        }
    }

    fn collect<Col: FromIterator<Self::Item>>(self) -> Col
    where
        Self: Sized,
    {
        match self {
            Either::Left(i) => i.collect(),
            Either::Right(i) => i.collect(),
        }
    }

    fn partition<Col, Fun>(self, f: Fun) -> (Col, Col)
    where
        Self: Sized,
        Col: Default + Extend<Self::Item>,
        Fun: FnMut(&Self::Item) -> bool,
    {
        match self {
            Either::Left(i) => i.partition(f),
            Either::Right(i) => i.partition(f),
        }
    }

    fn fold<Acc, Fun>(self, init: Acc, f: Fun) -> Acc
    where
        Self: Sized,
        Fun: FnMut(Acc, Self::Item) -> Acc,
    {
        match self {
            Either::Left(i) => i.fold(init, f),
            Either::Right(i) => i.fold(init, f),
        }
    }

    fn reduce<Fun>(self, f: Fun) -> Option<Self::Item>
    where
        Self: Sized,
        Fun: FnMut(Self::Item, Self::Item) -> Self::Item,
    {
        match self {
            Either::Left(i) => i.reduce(f),
            Either::Right(i) => i.reduce(f),
        }
    }

    fn all<Fun>(&mut self, f: Fun) -> bool
    where
        Self: Sized,
        Fun: FnMut(Self::Item) -> bool,
    {
        match self {
            Either::Left(i) => i.all(f),
            Either::Right(i) => i.all(f),
        }
    }

    fn any<Fun>(&mut self, f: Fun) -> bool
    where
        Self: Sized,
        Fun: FnMut(Self::Item) -> bool,
    {
        match self {
            Either::Left(i) => i.any(f),
            Either::Right(i) => i.any(f),
        }
    }

    fn find<Pre>(&mut self, predicate: Pre) -> Option<Self::Item>
    where
        Self: Sized,
        Pre: FnMut(&Self::Item) -> bool,
    {
        match self {
            Either::Left(i) => i.find(predicate),
            Either::Right(i) => i.find(predicate),
        }
    }

    fn find_map<Out, Fun>(&mut self, f: Fun) -> Option<Out>
    where
        Self: Sized,
        Fun: FnMut(Self::Item) -> Option<Out>,
    {
        match self {
            Either::Left(i) => i.find_map(f),
            Either::Right(i) => i.find_map(f),
        }
    }

    fn position<Pre>(&mut self, predicate: Pre) -> Option<usize>
    where
        Self: Sized,
        Pre: FnMut(Self::Item) -> bool,
    {
        match self {
            Either::Left(i) => i.position(predicate),
            Either::Right(i) => i.position(predicate),
        }
    }

    fn max(self) -> Option<Self::Item>
    where
        Self: Sized,
        Self::Item: Ord,
    {
        match self {
            Either::Left(i) => i.max(),
            Either::Right(i) => i.max(),
        }
    }

    fn min(self) -> Option<Self::Item>
    where
        Self: Sized,
        Self::Item: Ord,
    {
        match self {
            Either::Left(i) => i.min(),
            Either::Right(i) => i.min(),
        }
    }

    fn max_by_key<Key: Ord, Fun>(self, f: Fun) -> Option<Self::Item>
    where
        Self: Sized,
        Fun: FnMut(&Self::Item) -> Key,
    {
        match self {
            Either::Left(i) => i.max_by_key(f),
            Either::Right(i) => i.max_by_key(f),
        }
    }

    fn max_by<Cmp>(self, compare: Cmp) -> Option<Self::Item>
    where
        Self: Sized,
        Cmp: FnMut(&Self::Item, &Self::Item) -> Ordering,
    {
        match self {
            Either::Left(i) => i.max_by(compare),
            Either::Right(i) => i.max_by(compare),
        }
    }

    fn min_by_key<Key: Ord, Fun>(self, f: Fun) -> Option<Self::Item>
    where
        Self: Sized,
        Fun: FnMut(&Self::Item) -> Key,
    {
        match self {
            Either::Left(i) => i.min_by_key(f),
            Either::Right(i) => i.min_by_key(f),
        }
    }

    fn min_by<Cmp>(self, compare: Cmp) -> Option<Self::Item>
    where
        Self: Sized,
        Cmp: FnMut(&Self::Item, &Self::Item) -> Ordering,
    {
        match self {
            Either::Left(i) => i.min_by(compare),
            Either::Right(i) => i.min_by(compare),
        }
    }

    fn sum<Out>(self) -> Out
    where
        Self: Sized,
        Out: Sum<Self::Item>,
    {
        match self {
            Either::Left(i) => i.sum(),
            Either::Right(i) => i.sum(),
        }
    }

    fn product<Out>(self) -> Out
    where
        Self: Sized,
        Out: Product<Self::Item>,
    {
        match self {
            Either::Left(i) => i.product(),
            Either::Right(i) => i.product(),
        }
    }

    fn cmp<Other>(self, other: Other) -> Ordering
    where
        Other: IntoIterator<Item = Self::Item>,
        Self::Item: Ord,
        Self: Sized,
    {
        match self {
            Either::Left(i) => i.cmp(other),
            Either::Right(i) => i.cmp(other),
        }
    }

    fn partial_cmp<Other>(self, other: Other) -> Option<Ordering>
    where
        Other: IntoIterator,
        Self::Item: PartialOrd<Other::Item>,
        Self: Sized,
    {
        match self {
            Either::Left(i) => i.partial_cmp(other),
            Either::Right(i) => i.partial_cmp(other),
        }
    }

    fn is_sorted(self) -> bool
    where
        Self: Sized,
        Self::Item: PartialOrd,
    {
        match self {
            Either::Left(i) => i.is_sorted(),
            Either::Right(i) => i.is_sorted(),
        }
    }

    fn is_sorted_by<Cmp>(self, compare: Cmp) -> bool
    where
        Self: Sized,
        Cmp: FnMut(&Self::Item, &Self::Item) -> bool,
    {
        match self {
            Either::Left(i) => i.is_sorted_by(compare),
            Either::Right(i) => i.is_sorted_by(compare),
        }
    }

    fn is_sorted_by_key<Fun, Key>(self, f: Fun) -> bool
    where
        Self: Sized,
        Fun: FnMut(Self::Item) -> Key,
        Key: PartialOrd,
    {
        match self {
            Either::Left(i) => i.is_sorted_by_key(f),
            Either::Right(i) => i.is_sorted_by_key(f),
        }
    }
}

impl<Item, A, B> ExactSizeIterator for Either<A, B>
where
    A: ExactSizeIterator<Item = Item>,
    B: ExactSizeIterator<Item = Item>,
{
    fn len(&self) -> usize {
        match self {
            Either::Left(i) => i.len(),
            Either::Right(i) => i.len(),
        }
    }
}

impl<Item, A, B> DoubleEndedIterator for Either<A, B>
where
    A: DoubleEndedIterator<Item = Item>,
    B: DoubleEndedIterator<Item = Item>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        match self {
            Either::Left(i) => i.next_back(),
            Either::Right(i) => i.next_back(),
        }
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        match self {
            Either::Left(i) => i.nth_back(n),
            Either::Right(i) => i.nth_back(n),
        }
    }

    fn rfind<Pre>(&mut self, predicate: Pre) -> Option<Self::Item>
    where
        Pre: FnMut(&Self::Item) -> bool,
    {
        match self {
            Either::Left(i) => i.rfind(predicate),
            Either::Right(i) => i.rfind(predicate),
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

            fn size_hint(&self) -> (usize, Option<usize>) {
                match self {
                    $($name::$ty(i) => i.size_hint(),)*
                }
            }

            fn count(self) -> usize
            where
                Self: Sized,
            {
                match self {
                    $($name::$ty(i) => i.count(),)*
                }
            }

            fn last(self) -> Option<Self::Item>
            where
                Self: Sized,
            {
                match self {
                    $($name::$ty(i) => i.last(),)*
                }
            }

            fn nth(&mut self, n: usize) -> Option<Self::Item> {
                match self {
                    $($name::$ty(i) => i.nth(n),)*
                }
            }

            fn for_each<Fun>(self, f: Fun)
            where
                Self: Sized,
                Fun: FnMut(Self::Item),
            {
                match self {
                    $($name::$ty(i) => i.for_each(f),)*
                }
            }

            fn collect<Col: FromIterator<Self::Item>>(self) -> Col
            where
                Self: Sized,
            {
                match self {
                    $($name::$ty(i) => i.collect(),)*
                }
            }

            fn partition<Col, Fun>(self, f: Fun) -> (Col, Col)
            where
                Self: Sized,
                Col: Default + Extend<Self::Item>,
                Fun: FnMut(&Self::Item) -> bool,
            {
                match self {
                    $($name::$ty(i) => i.partition(f),)*
                }
            }

            fn fold<Acc, Fun>(self, init: Acc, f: Fun) -> Acc
            where
                Self: Sized,
                Fun: FnMut(Acc, Self::Item) -> Acc,
            {
                match self {
                    $($name::$ty(i) => i.fold(init, f),)*
                }
            }

            fn reduce<Fun>(self, f: Fun) -> Option<Self::Item>
            where
                Self: Sized,
                Fun: FnMut(Self::Item, Self::Item) -> Self::Item,
            {
                match self {
                    $($name::$ty(i) => i.reduce(f),)*
                }
            }

            fn all<Fun>(&mut self, f: Fun) -> bool
            where
                Self: Sized,
                Fun: FnMut(Self::Item) -> bool,
            {
                match self {
                    $($name::$ty(i) => i.all(f),)*
                }
            }

            fn any<Fun>(&mut self, f: Fun) -> bool
            where
                Self: Sized,
                Fun: FnMut(Self::Item) -> bool,
            {
                match self {
                    $($name::$ty(i) => i.any(f),)*
                }
            }

            fn find<Pre>(&mut self, predicate: Pre) -> Option<Self::Item>
            where
                Self: Sized,
                Pre: FnMut(&Self::Item) -> bool,
            {
                match self {
                    $($name::$ty(i) => i.find(predicate),)*
                }
            }

            fn find_map<Out, Fun>(&mut self, f: Fun) -> Option<Out>
            where
                Self: Sized,
                Fun: FnMut(Self::Item) -> Option<Out>,
            {
                match self {
                    $($name::$ty(i) => i.find_map(f),)*
                }
            }

            fn position<Pre>(&mut self, predicate: Pre) -> Option<usize>
            where
                Self: Sized,
                Pre: FnMut(Self::Item) -> bool,
            {
                match self {
                    $($name::$ty(i) => i.position(predicate),)*
                }
            }

            fn max(self) -> Option<Self::Item>
            where
                Self: Sized,
                Self::Item: Ord,
            {
                match self {
                    $($name::$ty(i) => i.max(),)*
                }
            }

            fn min(self) -> Option<Self::Item>
            where
                Self: Sized,
                Self::Item: Ord,
            {
                match self {
                    $($name::$ty(i) => i.min(),)*
                }
            }

            fn max_by_key<Key: Ord, Fun>(self, f: Fun) -> Option<Self::Item>
            where
                Self: Sized,
                Fun: FnMut(&Self::Item) -> Key,
            {
                match self {
                    $($name::$ty(i) => i.max_by_key(f),)*
                }
            }

            fn max_by<Cmp>(self, compare: Cmp) -> Option<Self::Item>
            where
                Self: Sized,
                Cmp: FnMut(&Self::Item, &Self::Item) -> Ordering,
            {
                match self {
                    $($name::$ty(i) => i.max_by(compare),)*
                }
            }

            fn min_by_key<Key: Ord, Fun>(self, f: Fun) -> Option<Self::Item>
            where
                Self: Sized,
                Fun: FnMut(&Self::Item) -> Key,
            {
                match self {
                    $($name::$ty(i) => i.min_by_key(f),)*
                }
            }

            fn min_by<Cmp>(self, compare: Cmp) -> Option<Self::Item>
            where
                Self: Sized,
                Cmp: FnMut(&Self::Item, &Self::Item) -> Ordering,
            {
                match self {
                    $($name::$ty(i) => i.min_by(compare),)*
                }
            }

            fn sum<Out>(self) -> Out
            where
                Self: Sized,
                Out: Sum<Self::Item>,
            {
                match self {
                    $($name::$ty(i) => i.sum(),)*
                }
            }

            fn product<Out>(self) -> Out
            where
                Self: Sized,
                Out: Product<Self::Item>,
            {
                match self {
                    $($name::$ty(i) => i.product(),)*
                }
            }

            fn cmp<Other>(self, other: Other) -> Ordering
            where
                Other: IntoIterator<Item = Self::Item>,
                Self::Item: Ord,
                Self: Sized,
            {
                match self {
                    $($name::$ty(i) => i.cmp(other),)*
                }
            }

            fn partial_cmp<Other>(self, other: Other) -> Option<Ordering>
            where
                Other: IntoIterator,
                Self::Item: PartialOrd<Other::Item>,
                Self: Sized,
            {
                match self {
                    $($name::$ty(i) => i.partial_cmp(other),)*
                }
            }

            fn is_sorted(self) -> bool
            where
                Self: Sized,
                Self::Item: PartialOrd,
            {
                match self {
                    $($name::$ty(i) => i.is_sorted(),)*
                }
            }

            fn is_sorted_by<Cmp>(self, compare: Cmp) -> bool
            where
                Self: Sized,
                Cmp: FnMut(&Self::Item, &Self::Item) -> bool,
            {
                match self {
                    $($name::$ty(i) => i.is_sorted_by(compare),)*
                }
            }

            fn is_sorted_by_key<Fun, Key>(self, f: Fun) -> bool
            where
                Self: Sized,
                Fun: FnMut(Self::Item) -> Key,
                Key: PartialOrd,
            {
                match self {
                    $($name::$ty(i) => i.is_sorted_by_key(f),)*
                }
            }
        }

        impl<Item, $($ty,)*> ExactSizeIterator for $name<$($ty,)*>
        where
            $($ty: ExactSizeIterator<Item = Item>,)*
        {
            fn len(&self) -> usize {
                match self {
                    $($name::$ty(i) => i.len(),)*
                }
            }
        }

        impl<Item, $($ty,)*> DoubleEndedIterator for $name<$($ty,)*>
        where
            $($ty: DoubleEndedIterator<Item = Item>,)*
        {
            fn next_back(&mut self) -> Option<Self::Item> {
                match self {
                    $($name::$ty(i) => i.next_back(),)*
                }
            }

            fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
                match self {
                    $($name::$ty(i) => i.nth_back(n),)*
                }
            }

            fn rfind<Pre>(&mut self, predicate: Pre) -> Option<Self::Item>
            where
                Pre: FnMut(&Self::Item) -> bool,
            {
                match self {
                    $($name::$ty(i) => i.rfind(predicate),)*
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
