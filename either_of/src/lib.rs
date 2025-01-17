#![no_std]
#![forbid(unsafe_code)]

//! Utilities for working with enumerated types that contain one of `2..n` other types.

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
