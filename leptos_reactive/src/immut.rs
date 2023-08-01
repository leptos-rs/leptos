//! This module contains the `Immutable` smart pointer,
//! which is used to store immutable references to values.
//! This is useful for storing, for example, strings.

use std::{
    borrow::{Borrow, Cow},
    fmt,
    hash::Hash,
    ops::{Add, Deref},
    rc::Rc,
};

/// An immutable smart pointer to a value.
pub enum Immutable<'a, T: ?Sized + 'a> {
    /// A static reference to a value.
    Borrowed(&'a T),
    /// A reference counted pointer to a value.
    Counted(Rc<T>),
}

impl<T: ?Sized> Deref for Immutable<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        match self {
            Immutable::Borrowed(v) => v,
            Immutable::Counted(v) => v,
        }
    }
}

impl<T: ?Sized> Borrow<T> for Immutable<'_, T> {
    #[inline]
    fn borrow(&self) -> &T {
        self.deref()
    }
}

impl<T: ?Sized> AsRef<T> for Immutable<'_, T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self.deref()
    }
}

impl Immutable<'_, str> {
    /// Returns a `&str` slice of this [`Immutable`].
    #[inline]
    pub fn as_str(&self) -> &str {
        self
    }
}

impl<T> Immutable<'_, [T]> {
    /// Returns a `&[T]` slice of this [`Immutable`].
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        self
    }
}

impl<T: ?Sized> Clone for Immutable<'_, T> {
    fn clone(&self) -> Self {
        match self {
            Immutable::Borrowed(v) => Immutable::Borrowed(v),
            Immutable::Counted(v) => Immutable::Counted(v.clone()),
        }
    }
}

impl<T: ?Sized> Default for Immutable<'_, T>
where
    T: ToOwned,
    T::Owned: Default,
    Rc<T>: From<T::Owned>,
{
    fn default() -> Self {
        Immutable::Counted(Rc::from(T::Owned::default()))
    }
}

impl<'a, 'b, A: ?Sized, B: ?Sized> PartialEq<Immutable<'b, B>>
    for Immutable<'a, A>
where
    A: PartialEq<B>,
{
    fn eq(&self, other: &Immutable<'b, B>) -> bool {
        **self == **other
    }
}

impl<T: ?Sized + Eq> Eq for Immutable<'_, T> {}

impl<'a, 'b, A: ?Sized, B: ?Sized> PartialOrd<Immutable<'b, B>>
    for Immutable<'a, A>
where
    A: PartialOrd<B>,
{
    fn partial_cmp(
        &self,
        other: &Immutable<'b, B>,
    ) -> Option<std::cmp::Ordering> {
        (**self).partial_cmp(&**other)
    }
}

impl<T: ?Sized + Ord> Ord for Immutable<'_, T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (**self).cmp(&**other)
    }
}

impl<T: ?Sized + Hash> Hash for Immutable<'_, T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (**self).hash(state)
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Immutable<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for Immutable<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: ?Sized> fmt::Pointer for Immutable<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&(&**self as *const T), f)
    }
}

impl<'a, T: ?Sized> From<&'a T> for Immutable<'a, T> {
    fn from(v: &'a T) -> Self {
        Immutable::Borrowed(v)
    }
}

impl<'a, T: ?Sized> From<Cow<'a, T>> for Immutable<'a, T>
where
    T: ToOwned,
    Rc<T>: From<T::Owned>,
{
    fn from(v: Cow<'a, T>) -> Self {
        match v {
            Cow::Borrowed(v) => Immutable::Borrowed(v),
            Cow::Owned(v) => Immutable::Counted(Rc::from(v)),
        }
    }
}

impl<'a, T: ?Sized> From<Immutable<'a, T>> for Cow<'a, T>
where
    T: ToOwned,
{
    fn from(value: Immutable<'a, T>) -> Self {
        match value {
            Immutable::Borrowed(v) => Cow::Borrowed(v),
            Immutable::Counted(v) => Cow::Owned(v.as_ref().to_owned()),
        }
    }
}

impl<T: ?Sized> From<Rc<T>> for Immutable<'_, T> {
    fn from(v: Rc<T>) -> Self {
        Immutable::Counted(v)
    }
}

impl<T: ?Sized> From<Box<T>> for Immutable<'_, T> {
    fn from(v: Box<T>) -> Self {
        Immutable::Counted(v.into())
    }
}

impl From<String> for Immutable<'_, str> {
    fn from(v: String) -> Self {
        Immutable::Counted(v.into())
    }
}

impl From<Immutable<'_, str>> for String {
    fn from(v: Immutable<'_, str>) -> Self {
        match v {
            Immutable::Borrowed(v) => v.to_owned(),
            Immutable::Counted(v) => v.as_ref().to_owned(),
        }
    }
}

impl<T> From<Vec<T>> for Immutable<'_, [T]> {
    fn from(v: Vec<T>) -> Self {
        Immutable::Counted(v.into())
    }
}

impl<T, const N: usize> From<[T; N]> for Immutable<'_, [T]> {
    fn from(v: [T; N]) -> Self {
        Immutable::Counted(Rc::from(v))
    }
}

impl<'a, T, const N: usize> From<&'a [T; N]> for Immutable<'a, [T]> {
    fn from(v: &'a [T; N]) -> Self {
        Immutable::Borrowed(v)
    }
}

impl<'a> From<Immutable<'a, str>> for Immutable<'a, [u8]> {
    fn from(v: Immutable<'a, str>) -> Self {
        match v {
            Immutable::Borrowed(v) => Immutable::Borrowed(v.as_bytes()),
            Immutable::Counted(v) => Immutable::Counted(v.into()),
        }
    }
}

macro_rules! impl_eq {
    ([$($g:tt)*] $((where $($w:tt)+))?, $lhs:ty, $rhs: ty) => {
        impl<$($g)*> PartialEq<$rhs> for $lhs
        $(where
            $($w)*)?
        {
            #[inline]
            fn eq(&self, other: &$rhs) -> bool {
                PartialEq::eq(&self[..], &other[..])
            }
        }

        impl<$($g)*> PartialEq<$lhs> for $rhs
        $(where
            $($w)*)?
        {
            #[inline]
            fn eq(&self, other: &$lhs) -> bool {
                PartialEq::eq(&self[..], &other[..])
            }
        }
    };
}

impl_eq!([], Immutable<'_, str>, str);
impl_eq!(['a, 'b], Immutable<'a, str>, &'b str);
impl_eq!([], Immutable<'_, str>, String);
impl_eq!(['a, 'b], Immutable<'a, str>, Cow<'b, str>);

impl_eq!([T: PartialEq], Immutable<'_, [T]>, [T]);
impl_eq!(['a, 'b, T: PartialEq], Immutable<'a, [T]>, &'b [T]);
impl_eq!([T: PartialEq], Immutable<'_, [T]>, Vec<T>);
impl_eq!(['a, 'b, T: PartialEq] (where [T]: ToOwned), Immutable<'a, [T]>, Cow<'b, [T]>);

impl<'a, 'b> Add<&'b str> for Immutable<'a, str> {
    type Output = Immutable<'static, str>;

    fn add(self, rhs: &'b str) -> Self::Output {
        Immutable::Counted(Rc::from(String::from(self) + rhs))
    }
}

impl<'a, 'b> Add<Cow<'b, str>> for Immutable<'a, str> {
    type Output = Immutable<'static, str>;

    fn add(self, rhs: Cow<'b, str>) -> Self::Output {
        Immutable::Counted(Rc::from(String::from(self) + rhs.as_ref()))
    }
}

impl<'a, 'b> Add<Immutable<'b, str>> for Immutable<'a, str> {
    type Output = Immutable<'static, str>;

    fn add(self, rhs: Immutable<'b, str>) -> Self::Output {
        Immutable::Counted(Rc::from(String::from(self) + rhs.as_ref()))
    }
}

impl<'a> FromIterator<Immutable<'a, str>> for String {
    fn from_iter<T: IntoIterator<Item = Immutable<'a, str>>>(iter: T) -> Self {
        iter.into_iter().map(String::from).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_fmt_should_display_quotes_for_strings() {
        let s: Immutable<str> = Immutable::Borrowed("hello");
        assert_eq!(format!("{:?}", s), "\"hello\"");
        let s: Immutable<str> = Immutable::Counted(Rc::from("hello"));
        assert_eq!(format!("{:?}", s), "\"hello\"");
    }

    #[test]
    fn partial_eq_should_compare_str_to_str() {
        let s: Immutable<str> = Immutable::Borrowed("hello");
        assert_eq!(s, "hello");
        assert_eq!("hello", s);
        assert_eq!(s, String::from("hello"));
        assert_eq!(String::from("hello"), s);
        assert_eq!(s, Cow::from("hello"));
        assert_eq!(Cow::from("hello"), s);
    }

    #[test]
    fn partial_eq_should_compare_slice_to_slice() {
        let s: Immutable<[i32]> = Immutable::Borrowed([1, 2, 3].as_slice());
        assert_eq!(s, [1, 2, 3].as_slice());
        assert_eq!([1, 2, 3].as_slice(), s);
        assert_eq!(s, vec![1, 2, 3]);
        assert_eq!(vec![1, 2, 3], s);
        assert_eq!(s, Cow::<'_, [i32]>::Borrowed(&[1, 2, 3]));
        assert_eq!(Cow::<'_, [i32]>::Borrowed(&[1, 2, 3]), s);
    }

    #[test]
    fn add_should_concatenate_strings() {
        let s: Immutable<str> = Immutable::Borrowed("hello");
        assert_eq!(s.clone() + " world", "hello world");
        assert_eq!(s.clone() + Cow::from(" world"), "hello world");
        assert_eq!(s + Immutable::from(" world"), "hello world");
    }

    #[test]
    fn as_str_should_return_a_str() {
        let s: Immutable<str> = Immutable::Borrowed("hello");
        assert_eq!(s.as_str(), "hello");
        let s: Immutable<str> = Immutable::Counted(Rc::from("hello"));
        assert_eq!(s.as_str(), "hello");
    }

    #[test]
    fn as_slice_should_return_a_slice() {
        let s: Immutable<[i32]> = Immutable::Borrowed([1, 2, 3].as_slice());
        assert_eq!(s.as_slice(), [1, 2, 3].as_slice());
        let s: Immutable<[i32]> = Immutable::Counted(Rc::from([1, 2, 3]));
        assert_eq!(s.as_slice(), [1, 2, 3].as_slice());
    }
}
