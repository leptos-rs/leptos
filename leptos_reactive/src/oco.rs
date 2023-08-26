//! This module contains the `Oco` (Owned Clones Once) smart pointer,
//! which is used to store immutable references to values.
//! This is useful for storing, for example, strings.

use std::{
    borrow::{Borrow, Cow},
    ffi::{CStr, OsStr},
    fmt,
    hash::Hash,
    ops::{Add, Deref},
    path::Path,
    rc::Rc,
};

/// "Owned Clones Once" - a smart pointer that can be either a reference,
/// an owned value, or a reference counted pointer. This is useful for
/// storing immutable values, such as strings, in a way that is cheap to
/// clone and pass around.
///
/// The `Clone` implementation is amortized `O(1)`. Cloning the [`Oco::Borrowed`]
/// variant simply copies the references (`O(1)`). Cloning the [`Oco::Counted`]
/// variant increments a reference count (`O(1)`). Cloning the [`Oco::Owned`]
/// variant upgrades it to [`Oco::Counted`], which requires an `O(n)` clone of the
/// data, but all subsequent clones will be `O(1)`.
pub enum Oco<'a, T: ?Sized + ToOwned + 'a> {
    /// A static reference to a value.
    Borrowed(&'a T),
    /// A reference counted pointer to a value.
    Counted(Rc<T>),
    /// An owned value.
    Owned(<T as ToOwned>::Owned),
}

impl<T: ?Sized + ToOwned> Oco<'_, T> {
    /// Converts the value into an owned value.
    pub fn into_owned(self) -> <T as ToOwned>::Owned {
        match self {
            Oco::Borrowed(v) => v.to_owned(),
            Oco::Counted(v) => v.as_ref().to_owned(),
            Oco::Owned(v) => v,
        }
    }

    /// Checks if the value is [`Oco::Borrowed`].
    /// # Examples
    /// ```
    /// # use std::rc::Rc;
    /// # use leptos_reactive::oco::Oco;
    /// assert!(Oco::<str>::Borrowed("Hello").is_borrowed());
    /// assert!(!Oco::<str>::Counted(Rc::from("Hello")).is_borrowed());
    /// assert!(!Oco::<str>::Owned("Hello".to_string()).is_borrowed());
    /// ```
    pub const fn is_borrowed(&self) -> bool {
        matches!(self, Oco::Borrowed(_))
    }

    /// Checks if the value is [`Oco::Counted`].
    /// # Examples
    /// ```
    /// # use std::rc::Rc;
    /// # use leptos_reactive::oco::Oco;
    /// assert!(Oco::<str>::Counted(Rc::from("Hello")).is_counted());
    /// assert!(!Oco::<str>::Borrowed("Hello").is_counted());
    /// assert!(!Oco::<str>::Owned("Hello".to_string()).is_counted());
    /// ```
    pub const fn is_counted(&self) -> bool {
        matches!(self, Oco::Counted(_))
    }

    /// Checks if the value is [`Oco::Owned`].
    /// # Examples
    /// ```
    /// # use std::rc::Rc;
    /// # use leptos_reactive::oco::Oco;
    /// assert!(Oco::<str>::Owned("Hello".to_string()).is_owned());
    /// assert!(!Oco::<str>::Borrowed("Hello").is_owned());
    /// assert!(!Oco::<str>::Counted(Rc::from("Hello")).is_owned());
    /// ```
    pub const fn is_owned(&self) -> bool {
        matches!(self, Oco::Owned(_))
    }
}

impl<T: ?Sized + ToOwned> Deref for Oco<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        match self {
            Oco::Borrowed(v) => v,
            Oco::Owned(v) => v.borrow(),
            Oco::Counted(v) => v,
        }
    }
}

impl<T: ?Sized + ToOwned> Borrow<T> for Oco<'_, T> {
    #[inline(always)]
    fn borrow(&self) -> &T {
        self.deref()
    }
}

impl<T: ?Sized + ToOwned> AsRef<T> for Oco<'_, T> {
    #[inline(always)]
    fn as_ref(&self) -> &T {
        self.deref()
    }
}

impl AsRef<Path> for Oco<'_, str> {
    #[inline(always)]
    fn as_ref(&self) -> &Path {
        self.as_str().as_ref()
    }
}

impl AsRef<Path> for Oco<'_, OsStr> {
    #[inline(always)]
    fn as_ref(&self) -> &Path {
        self.as_os_str().as_ref()
    }
}

// --------------------------------------
// pub fn as_{slice}(&self) -> &{slice}
// --------------------------------------

impl Oco<'_, str> {
    /// Returns a `&str` slice of this [`Oco`].
    /// # Examples
    /// ```
    /// # use leptos_reactive::oco::Oco;
    /// let oco = Oco::<str>::Borrowed("Hello");
    /// let s: &str = oco.as_str();
    /// assert_eq!(s, "Hello");
    /// ```
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        self
    }
}

impl Oco<'_, CStr> {
    /// Returns a `&CStr` slice of this [`Oco`].
    /// # Examples
    /// ```
    /// # use leptos_reactive::oco::Oco;
    /// use std::ffi::CStr;
    ///
    /// let oco =
    ///     Oco::<CStr>::Borrowed(CStr::from_bytes_with_nul(b"Hello\0").unwrap());
    /// let s: &CStr = oco.as_c_str();
    /// assert_eq!(s, CStr::from_bytes_with_nul(b"Hello\0").unwrap());
    /// ```
    #[inline(always)]
    pub fn as_c_str(&self) -> &CStr {
        self
    }
}

impl Oco<'_, OsStr> {
    /// Returns a `&OsStr` slice of this [`Oco`].
    /// # Examples
    /// ```
    /// # use leptos_reactive::oco::Oco;
    /// use std::ffi::OsStr;
    ///
    /// let oco = Oco::<OsStr>::Borrowed(OsStr::new("Hello"));
    /// let s: &OsStr = oco.as_os_str();
    /// assert_eq!(s, OsStr::new("Hello"));
    /// ```
    #[inline(always)]
    pub fn as_os_str(&self) -> &OsStr {
        self
    }
}

impl Oco<'_, Path> {
    /// Returns a `&Path` slice of this [`Oco`].
    /// # Examples
    /// ```
    /// # use leptos_reactive::oco::Oco;
    /// use std::path::Path;
    ///
    /// let oco = Oco::<Path>::Borrowed(Path::new("Hello"));
    /// let s: &Path = oco.as_path();
    /// assert_eq!(s, Path::new("Hello"));
    /// ```
    #[inline(always)]
    pub fn as_path(&self) -> &Path {
        self
    }
}

impl<T> Oco<'_, [T]>
where
    [T]: ToOwned,
{
    /// Returns a `&[T]` slice of this [`Oco`].
    /// # Examples
    /// ```
    /// # use leptos_reactive::oco::Oco;
    /// let oco = Oco::<[u8]>::Borrowed(b"Hello");
    /// let s: &[u8] = oco.as_slice();
    /// assert_eq!(s, b"Hello");
    /// ```
    #[inline(always)]
    pub fn as_slice(&self) -> &[T] {
        self
    }
}

// ------------------------------------------------------------------------------------------------------
// Cloning (has to be implemented manually because of the `Rc<T>: From<&<T as ToOwned>::Owned>` bound)
// ------------------------------------------------------------------------------------------------------

impl Clone for Oco<'_, str> {
    /// Returns a new [`Oco`] with the same value as this one.
    /// If the value is [`Oco::Owned`], this will convert it into
    /// [`Oco::Counted`], so that the next clone will be O(1).
    /// # Examples
    /// ```
    /// # use leptos_reactive::oco::Oco;
    /// let oco = Oco::<str>::Owned("Hello".to_string());
    /// let oco2 = oco.clone();
    /// assert_eq!(oco, oco2);
    /// assert!(oco2.is_counted());
    /// ```
    fn clone(&self) -> Self {
        match self {
            Oco::Borrowed(v) => Oco::Borrowed(v),
            Oco::Counted(v) => Oco::Counted(v.clone()),
            Oco::Owned(v) => Oco::Counted(Rc::<str>::from(v.as_str())),
        }
    }
}

impl Clone for Oco<'_, CStr> {
    /// Returns a new [`Oco`] with the same value as this one.
    /// If the value is [`Oco::Owned`], this will convert it into
    /// [`Oco::Counted`], so that the next clone will be O(1).
    /// # Examples
    /// ```
    /// # use leptos_reactive::oco::Oco;
    /// use std::ffi::CStr;
    ///
    /// let oco = Oco::<CStr>::Owned(
    ///     CStr::from_bytes_with_nul(b"Hello\0").unwrap().to_owned(),
    /// );
    /// let oco2 = oco.clone();
    /// assert_eq!(oco, oco2);
    /// assert!(oco2.is_counted());
    /// ```
    fn clone(&self) -> Self {
        match self {
            Oco::Borrowed(v) => Oco::Borrowed(v),
            Oco::Counted(v) => Oco::Counted(v.clone()),
            Oco::Owned(v) => Oco::Counted(Rc::<CStr>::from(v.as_c_str())),
        }
    }
}

impl Clone for Oco<'_, OsStr> {
    /// Returns a new [`Oco`] with the same value as this one.
    /// If the value is [`Oco::Owned`], this will convert it into
    /// [`Oco::Counted`], so that the next clone will be O(1).
    /// # Examples
    /// ```
    /// # use leptos_reactive::oco::Oco;
    /// use std::ffi::OsStr;
    ///
    /// let oco = Oco::<OsStr>::Owned(OsStr::new("Hello").to_owned());
    /// let oco2 = oco.clone();
    /// assert_eq!(oco, oco2);
    /// assert!(oco2.is_counted());
    /// ```
    fn clone(&self) -> Self {
        match self {
            Oco::Borrowed(v) => Oco::Borrowed(v),
            Oco::Counted(v) => Oco::Counted(v.clone()),
            Oco::Owned(v) => Oco::Counted(Rc::<OsStr>::from(v.as_os_str())),
        }
    }
}

impl Clone for Oco<'_, Path> {
    /// Returns a new [`Oco`] with the same value as this one.
    /// If the value is [`Oco::Owned`], this will convert it into
    /// [`Oco::Counted`], so that the next clone will be O(1).
    /// # Examples
    /// ```
    /// # use leptos_reactive::oco::Oco;
    /// use std::path::Path;
    ///
    /// let oco = Oco::<Path>::Owned(Path::new("Hello").to_owned());
    /// let oco2 = oco.clone();
    /// assert_eq!(oco, oco2);
    /// assert!(oco2.is_counted());
    /// ```
    fn clone(&self) -> Self {
        match self {
            Oco::Borrowed(v) => Oco::Borrowed(v),
            Oco::Counted(v) => Oco::Counted(v.clone()),
            Oco::Owned(v) => Oco::Counted(Rc::<Path>::from(v.as_path())),
        }
    }
}

impl<T: Clone> Clone for Oco<'_, [T]>
where
    [T]: ToOwned<Owned = Vec<T>>,
{
    /// Returns a new [`Oco`] with the same value as this one.
    /// If the value is [`Oco::Owned`], this will convert it into
    /// [`Oco::Counted`], so that the next clone will be O(1).
    /// # Examples
    /// ```
    /// # use leptos_reactive::oco::Oco;
    /// let oco = Oco::<[i32]>::Owned(vec![1, 2, 3]);
    /// let oco2 = oco.clone();
    /// assert_eq!(oco, oco2);
    /// assert!(oco2.is_counted());
    /// ```
    fn clone(&self) -> Self {
        match self {
            Oco::Borrowed(v) => Oco::Borrowed(v),
            Oco::Counted(v) => Oco::Counted(v.clone()),
            Oco::Owned(v) => Oco::Counted(Rc::<[T]>::from(v.as_slice())),
        }
    }
}

impl<T: ?Sized> Default for Oco<'_, T>
where
    T: ToOwned,
    T::Owned: Default,
{
    fn default() -> Self {
        Oco::Owned(T::Owned::default())
    }
}

impl<'a, 'b, A: ?Sized, B: ?Sized> PartialEq<Oco<'b, B>> for Oco<'a, A>
where
    A: PartialEq<B>,
    A: ToOwned,
    B: ToOwned,
{
    fn eq(&self, other: &Oco<'b, B>) -> bool {
        **self == **other
    }
}

impl<T: ?Sized + ToOwned + Eq> Eq for Oco<'_, T> {}

impl<'a, 'b, A: ?Sized, B: ?Sized> PartialOrd<Oco<'b, B>> for Oco<'a, A>
where
    A: PartialOrd<B>,
    A: ToOwned,
    B: ToOwned,
{
    fn partial_cmp(&self, other: &Oco<'b, B>) -> Option<std::cmp::Ordering> {
        (**self).partial_cmp(&**other)
    }
}

impl<T: ?Sized + Ord> Ord for Oco<'_, T>
where
    T: ToOwned,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (**self).cmp(&**other)
    }
}

impl<T: ?Sized + Hash> Hash for Oco<'_, T>
where
    T: ToOwned,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (**self).hash(state)
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Oco<'_, T>
where
    T: ToOwned,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for Oco<'_, T>
where
    T: ToOwned,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<'a, T: ?Sized> From<&'a T> for Oco<'a, T>
where
    T: ToOwned,
{
    fn from(v: &'a T) -> Self {
        Oco::Borrowed(v)
    }
}

impl<'a, T: ?Sized> From<Cow<'a, T>> for Oco<'a, T>
where
    T: ToOwned,
{
    fn from(v: Cow<'a, T>) -> Self {
        match v {
            Cow::Borrowed(v) => Oco::Borrowed(v),
            Cow::Owned(v) => Oco::Owned(v),
        }
    }
}

impl<'a, T: ?Sized> From<Oco<'a, T>> for Cow<'a, T>
where
    T: ToOwned,
{
    fn from(value: Oco<'a, T>) -> Self {
        match value {
            Oco::Borrowed(v) => Cow::Borrowed(v),
            Oco::Owned(v) => Cow::Owned(v),
            Oco::Counted(v) => Cow::Owned(v.as_ref().to_owned()),
        }
    }
}

impl<T: ?Sized> From<Rc<T>> for Oco<'_, T>
where
    T: ToOwned,
{
    fn from(v: Rc<T>) -> Self {
        Oco::Counted(v)
    }
}

impl<T: ?Sized> From<Box<T>> for Oco<'_, T>
where
    T: ToOwned,
{
    fn from(v: Box<T>) -> Self {
        Oco::Counted(v.into())
    }
}

impl From<String> for Oco<'_, str> {
    fn from(v: String) -> Self {
        Oco::Owned(v)
    }
}

impl From<Oco<'_, str>> for String {
    fn from(v: Oco<'_, str>) -> Self {
        match v {
            Oco::Borrowed(v) => v.to_owned(),
            Oco::Counted(v) => v.as_ref().to_owned(),
            Oco::Owned(v) => v,
        }
    }
}

impl<T> From<Vec<T>> for Oco<'_, [T]>
where
    [T]: ToOwned<Owned = Vec<T>>,
{
    fn from(v: Vec<T>) -> Self {
        Oco::Owned(v)
    }
}

impl<'a, T, const N: usize> From<&'a [T; N]> for Oco<'a, [T]>
where
    [T]: ToOwned,
{
    fn from(v: &'a [T; N]) -> Self {
        Oco::Borrowed(v)
    }
}

impl<'a> From<Oco<'a, str>> for Oco<'a, [u8]> {
    fn from(v: Oco<'a, str>) -> Self {
        match v {
            Oco::Borrowed(v) => Oco::Borrowed(v.as_bytes()),
            Oco::Owned(v) => Oco::Owned(v.into_bytes()),
            Oco::Counted(v) => Oco::Counted(v.into()),
        }
    }
}

/// Error returned from [`Oco::try_from`] for unsuccessful
/// conversion from `Oco<'_, [u8]>` to `Oco<'_, str>`.
#[derive(Debug, Clone, thiserror::Error)]
#[error("invalid utf-8 sequence: {_0}")]
pub enum FromUtf8Error {
    /// Error for conversion of [`Oco::Borrowed`] and [`Oco::Counted`] variants
    /// (`&[u8]` to `&str`).
    #[error("{_0}")]
    StrFromBytes(
        #[source]
        #[from]
        std::str::Utf8Error,
    ),
    /// Error for conversion of [`Oco::Owned`] variant (`Vec<u8>` to `String`).
    #[error("{_0}")]
    StringFromBytes(
        #[source]
        #[from]
        std::string::FromUtf8Error,
    ),
}

macro_rules! impl_slice_eq {
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

impl_slice_eq!([], Oco<'_, str>, str);
impl_slice_eq!(['a, 'b], Oco<'a, str>, &'b str);
impl_slice_eq!([], Oco<'_, str>, String);
impl_slice_eq!(['a, 'b], Oco<'a, str>, Cow<'b, str>);

impl_slice_eq!([T: PartialEq] (where [T]: ToOwned), Oco<'_, [T]>, [T]);
impl_slice_eq!(['a, 'b, T: PartialEq] (where [T]: ToOwned), Oco<'a, [T]>, &'b [T]);
impl_slice_eq!([T: PartialEq] (where [T]: ToOwned), Oco<'_, [T]>, Vec<T>);
impl_slice_eq!(['a, 'b, T: PartialEq] (where [T]: ToOwned), Oco<'a, [T]>, Cow<'b, [T]>);

impl<'a, 'b> Add<&'b str> for Oco<'a, str> {
    type Output = Oco<'static, str>;

    fn add(self, rhs: &'b str) -> Self::Output {
        Oco::Owned(String::from(self) + rhs)
    }
}

impl<'a, 'b> Add<Cow<'b, str>> for Oco<'a, str> {
    type Output = Oco<'static, str>;

    fn add(self, rhs: Cow<'b, str>) -> Self::Output {
        Oco::Owned(String::from(self) + rhs.as_ref())
    }
}

impl<'a, 'b> Add<Oco<'b, str>> for Oco<'a, str> {
    type Output = Oco<'static, str>;

    fn add(self, rhs: Oco<'b, str>) -> Self::Output {
        Oco::Owned(String::from(self) + rhs.as_ref())
    }
}

impl<'a> FromIterator<Oco<'a, str>> for String {
    fn from_iter<T: IntoIterator<Item = Oco<'a, str>>>(iter: T) -> Self {
        iter.into_iter().fold(String::new(), |mut acc, item| {
            acc.push_str(item.as_ref());
            acc
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_fmt_should_display_quotes_for_strings() {
        let s: Oco<str> = Oco::Borrowed("hello");
        assert_eq!(format!("{:?}", s), "\"hello\"");
        let s: Oco<str> = Oco::Counted(Rc::from("hello"));
        assert_eq!(format!("{:?}", s), "\"hello\"");
    }

    #[test]
    fn partial_eq_should_compare_str_to_str() {
        let s: Oco<str> = Oco::Borrowed("hello");
        assert_eq!(s, "hello");
        assert_eq!("hello", s);
        assert_eq!(s, String::from("hello"));
        assert_eq!(String::from("hello"), s);
        assert_eq!(s, Cow::from("hello"));
        assert_eq!(Cow::from("hello"), s);
    }

    #[test]
    fn partial_eq_should_compare_slice_to_slice() {
        let s: Oco<[i32]> = Oco::Borrowed([1, 2, 3].as_slice());
        assert_eq!(s, [1, 2, 3].as_slice());
        assert_eq!([1, 2, 3].as_slice(), s);
        assert_eq!(s, vec![1, 2, 3]);
        assert_eq!(vec![1, 2, 3], s);
        assert_eq!(s, Cow::<'_, [i32]>::Borrowed(&[1, 2, 3]));
        assert_eq!(Cow::<'_, [i32]>::Borrowed(&[1, 2, 3]), s);
    }

    #[test]
    fn add_should_concatenate_strings() {
        let s: Oco<str> = Oco::Borrowed("hello");
        assert_eq!(s.clone() + " world", "hello world");
        assert_eq!(s.clone() + Cow::from(" world"), "hello world");
        assert_eq!(s + Oco::from(" world"), "hello world");
    }

    #[test]
    fn as_str_should_return_a_str() {
        let s: Oco<str> = Oco::Borrowed("hello");
        assert_eq!(s.as_str(), "hello");
        let s: Oco<str> = Oco::Counted(Rc::from("hello"));
        assert_eq!(s.as_str(), "hello");
    }

    #[test]
    fn as_slice_should_return_a_slice() {
        let s: Oco<[i32]> = Oco::Borrowed([1, 2, 3].as_slice());
        assert_eq!(s.as_slice(), [1, 2, 3].as_slice());
        let s: Oco<[i32]> = Oco::Counted(Rc::from([1, 2, 3]));
        assert_eq!(s.as_slice(), [1, 2, 3].as_slice());
    }

    #[test]
    fn default_for_str_should_return_an_empty_string() {
        let s: Oco<str> = Default::default();
        assert!(s.is_empty());
    }

    #[test]
    fn default_for_slice_should_return_an_empty_slice() {
        let s: Oco<[i32]> = Default::default();
        assert!(s.is_empty());
    }

    #[test]
    fn default_for_any_option_should_return_none() {
        let s: Oco<Option<i32>> = Default::default();
        assert!(s.is_none());
    }

    #[test]
    fn cloned_owned_string_should_become_counted_str() {
        let s: Oco<str> = Oco::Owned(String::from("hello"));
        assert!(s.clone().is_counted());
    }

    #[test]
    fn cloned_borrowed_str_should_remain_borrowed_str() {
        let s: Oco<str> = Oco::Borrowed("hello");
        assert!(s.clone().is_borrowed());
    }

    #[test]
    fn cloned_counted_str_should_remain_counted_str() {
        let s: Oco<str> = Oco::Counted(Rc::from("hello"));
        assert!(s.clone().is_counted());
    }
}
