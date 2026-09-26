//! Defines [`Oco<'a, T>`], an "Owned Clones Once" smart pointer,
//! which is used to store immutable references to values.
//! This is useful for storing, for example, strings.
//!
//! Imagine this as an alternative to [`Cow`] with an additional, reference-counted
//! branch.
//!
//! ```rust
//! use oco_ref::Oco;
//! use std::sync::Arc;
//!
//! let static_str = "foo";
//! let rc_str: Arc<str> = "bar".into();
//! let owned_str: String = "baz".into();
//!
//! fn uses_oco(value: impl Into<Oco<'static, str>>) {
//!     let mut value = value.into();
//!
//!     // ensures that the value is either a reference, or reference-counted
//!     // O(n) at worst
//!     let clone1 = value.clone_inplace();
//!
//!     // these subsequent clones are O(1)
//!     let clone2 = value.clone();
//!     let clone3 = value.clone();
//! }
//!
//! uses_oco(static_str);
//! uses_oco(rc_str);
//! uses_oco(owned_str);
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    borrow::{Borrow, Cow},
    ffi::{CStr, CString, OsStr, OsString},
    fmt,
    hash::Hash,
    ops::{Add, Deref},
    path::{Path, PathBuf},
    sync::Arc,
};

/// "Owned Clones Once": a smart pointer that can be either a reference,
/// an owned value, or a reference-counted pointer. This is useful for
/// storing immutable values, such as strings, in a way that is cheap to
/// clone and pass around.
///
/// The cost of the `Clone` implementation depends on the branch.  Cloning the [`Oco::Borrowed`]
/// variant simply copies the references (`O(1)`). Cloning the [`Oco::Counted`]
/// variant increments a reference count (`O(1)`). Cloning the [`Oco::Owned`]
/// variant requires an `O(n)` clone of the data.
///
/// For an amortized `O(1)` clone, you can use [`Oco::clone_inplace()`]. Using this method,
/// [`Oco::Borrowed`] and [`Oco::Counted`] are still `O(1)`. [`Oco::Owned`] does a single `O(n)`
/// clone, but converts the object to the [`Oco::Counted`] branch, which means future clones will
/// be `O(1)`.
///
/// In general, you'll either want to call `clone_inplace()` once, before sharing the `Oco` with
/// other parts of your application (so that all future clones are `O(1)`), or simply use this as
/// if it is a [`Cow`] with an additional branch for reference-counted values.
pub enum Oco<'a, T: ?Sized + ToOwned + 'a> {
    /// A static reference to a value.
    Borrowed(&'a T),
    /// A reference counted pointer to a value.
    Counted(Arc<T>),
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
    /// # use std::sync::Arc;
    /// # use oco_ref::Oco;
    /// assert!(Oco::<str>::Borrowed("Hello").is_borrowed());
    /// assert!(!Oco::<str>::Counted(Arc::from("Hello")).is_borrowed());
    /// assert!(!Oco::<str>::Owned("Hello".to_string()).is_borrowed());
    /// ```
    pub const fn is_borrowed(&self) -> bool {
        matches!(self, Oco::Borrowed(_))
    }

    /// Checks if the value is [`Oco::Counted`].
    /// # Examples
    /// ```
    /// # use std::sync::Arc;
    /// # use oco_ref::Oco;
    /// assert!(Oco::<str>::Counted(Arc::from("Hello")).is_counted());
    /// assert!(!Oco::<str>::Borrowed("Hello").is_counted());
    /// assert!(!Oco::<str>::Owned("Hello".to_string()).is_counted());
    /// ```
    pub const fn is_counted(&self) -> bool {
        matches!(self, Oco::Counted(_))
    }

    /// Checks if the value is [`Oco::Owned`].
    /// # Examples
    /// ```
    /// # use std::sync::Arc;
    /// # use oco_ref::Oco;
    /// assert!(Oco::<str>::Owned("Hello".to_string()).is_owned());
    /// assert!(!Oco::<str>::Borrowed("Hello").is_owned());
    /// assert!(!Oco::<str>::Counted(Arc::from("Hello")).is_owned());
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
    /// # use oco_ref::Oco;
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
    /// # use oco_ref::Oco;
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
    /// # use oco_ref::Oco;
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
    /// # use oco_ref::Oco;
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
    /// # use oco_ref::Oco;
    /// let oco = Oco::<[u8]>::Borrowed(b"Hello");
    /// let s: &[u8] = oco.as_slice();
    /// assert_eq!(s, b"Hello");
    /// ```
    #[inline(always)]
    pub fn as_slice(&self) -> &[T] {
        self
    }
}

impl<'a, T> Clone for Oco<'a, T>
where
    T: ?Sized + ToOwned + 'a,
    for<'b> Arc<T>: From<&'b T>,
{
    /// Returns a new [`Oco`] with the same value as this one.
    /// If the value is [`Oco::Owned`], this will convert it into
    /// [`Oco::Counted`], so that the next clone will be O(1).
    /// # Examples
    /// [`String`] :
    /// ```
    /// # use oco_ref::Oco;
    /// let oco = Oco::<str>::Owned("Hello".to_string());
    /// let oco2 = oco.clone();
    /// assert_eq!(oco, oco2);
    /// assert!(oco2.is_counted());
    /// ```
    /// [`Vec`] :
    /// ```
    /// # use oco_ref::Oco;
    /// let oco = Oco::<[u8]>::Owned(b"Hello".to_vec());
    /// let oco2 = oco.clone();
    /// assert_eq!(oco, oco2);
    /// assert!(oco2.is_counted());
    /// ```
    fn clone(&self) -> Self {
        match self {
            Self::Borrowed(v) => Self::Borrowed(v),
            Self::Counted(v) => Self::Counted(Arc::clone(v)),
            Self::Owned(v) => Self::Counted(Arc::from(v.borrow())),
        }
    }
}

impl<'a, T> Oco<'a, T>
where
    T: ?Sized + ToOwned + 'a,
    for<'b> Arc<T>: From<&'b T>,
{
    /// Upgrades the value in place, by converting into [`Oco::Counted`] if it
    /// was previously [`Oco::Owned`].
    /// # Examples
    /// ```
    /// # use oco_ref::Oco;
    /// let mut oco1 = Oco::<str>::Owned("Hello".to_string());
    /// assert!(oco1.is_owned());
    /// oco1.upgrade_inplace();
    /// assert!(oco1.is_counted());
    /// ```
    pub fn upgrade_inplace(&mut self) {
        if let Self::Owned(v) = &*self {
            let rc = Arc::from(v.borrow());
            *self = Self::Counted(rc);
        }
    }

    /// Clones the value with inplace conversion into [`Oco::Counted`] if it
    /// was previously [`Oco::Owned`].
    /// # Examples
    /// ```
    /// # use oco_ref::Oco;
    /// let mut oco1 = Oco::<str>::Owned("Hello".to_string());
    /// let oco2 = oco1.clone_inplace();
    /// assert_eq!(oco1, oco2);
    /// assert!(oco1.is_counted());
    /// assert!(oco2.is_counted());
    /// ```
    pub fn clone_inplace(&mut self) -> Self {
        match &*self {
            Self::Borrowed(v) => Self::Borrowed(v),
            Self::Counted(v) => Self::Counted(Arc::clone(v)),
            Self::Owned(v) => {
                let rc = Arc::from(v.borrow());
                *self = Self::Counted(rc.clone());
                Self::Counted(rc)
            }
        }
    }
}

// `Default` is specialised per unsized target so that the empty value
// can be served from `'static` borrow without allocation. The blanket
// `impl<T: ToOwned> Default for Oco<'_, T>` was removed because it
// returned `Oco::Owned(T::Owned::default())` and forced an `O(n)`
// upgrade on the first `.clone()` even though every empty value has
// a `&'static` representation.

impl Default for Oco<'_, str> {
    /// Returns an empty [`Oco::Borrowed`] string with `'static` lifetime.
    /// No allocation, and subsequent `.clone()` calls are `O(1)`.
    fn default() -> Self {
        Oco::Borrowed("")
    }
}

impl<T> Default for Oco<'_, [T]>
where
    [T]: ToOwned,
{
    /// Returns an empty [`Oco::Borrowed`] slice with `'static` lifetime.
    /// No allocation, and subsequent `.clone()` calls are `O(1)`.
    fn default() -> Self {
        Oco::Borrowed(&[])
    }
}

impl Default for Oco<'_, CStr> {
    /// Returns an empty [`Oco::Borrowed`] C string with `'static` lifetime.
    /// No allocation, and subsequent `.clone()` calls are `O(1)`.
    fn default() -> Self {
        Oco::Borrowed(c"")
    }
}

impl Default for Oco<'_, OsStr> {
    /// Returns an empty [`Oco::Borrowed`] OS string with `'static` lifetime.
    /// No allocation, and subsequent `.clone()` calls are `O(1)`.
    fn default() -> Self {
        Oco::Borrowed(OsStr::new(""))
    }
}

impl Default for Oco<'_, Path> {
    /// Returns an empty [`Oco::Borrowed`] path with `'static` lifetime.
    /// No allocation, and subsequent `.clone()` calls are `O(1)`.
    fn default() -> Self {
        Oco::Borrowed(Path::new(""))
    }
}

impl<'b, A: ?Sized, B: ?Sized> PartialEq<Oco<'b, B>> for Oco<'_, A>
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

impl<'b, A: ?Sized, B: ?Sized> PartialOrd<Oco<'b, B>> for Oco<'_, A>
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

impl<T: ?Sized> From<Arc<T>> for Oco<'_, T>
where
    T: ToOwned,
{
    fn from(v: Arc<T>) -> Self {
        Oco::Counted(v)
    }
}

// `From<Box<T>>` is provided for the standard unsized targets (`str`,
// `[T]`, `CStr`, `OsStr`, `Path`) and always materialises into
// [`Oco::Owned`], matching [`From<String>`] and [`From<Vec<T>>`]. Each
// conversion reuses the boxed allocation as the corresponding owned
// value (zero extra payload copy) and lets the user opt into
// reference-counting explicitly via [`Oco::clone_inplace`] when
// sharing is desired.

impl From<Box<str>> for Oco<'_, str> {
    fn from(v: Box<str>) -> Self {
        Oco::Owned(String::from(v))
    }
}

impl<T> From<Box<[T]>> for Oco<'_, [T]>
where
    [T]: ToOwned<Owned = Vec<T>>,
{
    fn from(v: Box<[T]>) -> Self {
        Oco::Owned(Vec::from(v))
    }
}

impl From<Box<CStr>> for Oco<'_, CStr> {
    fn from(v: Box<CStr>) -> Self {
        Oco::Owned(CString::from(v))
    }
}

impl From<Box<OsStr>> for Oco<'_, OsStr> {
    fn from(v: Box<OsStr>) -> Self {
        Oco::Owned(OsString::from(v))
    }
}

impl From<Box<Path>> for Oco<'_, Path> {
    fn from(v: Box<Path>) -> Self {
        Oco::Owned(PathBuf::from(v))
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

/// Error returned from `Oco::try_from` for unsuccessful
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

impl<'b> Add<&'b str> for Oco<'_, str> {
    type Output = Oco<'static, str>;

    /// Concatenates `self` and `rhs` into a new `Oco<'static, str>`.
    ///
    /// In the general case this allocates a fresh `String`; the
    /// `'static` lifetime on the output refers to that freshly owned
    /// buffer, not to borrowed `'static` input. Fast-paths that avoid
    /// the allocation:
    ///
    /// - both sides empty → returns `Oco::Borrowed("")`.
    /// - `rhs` empty and `self` is [`Oco::Counted`] or [`Oco::Owned`] →
    ///   returns `self` unchanged.
    fn add(self, rhs: &'b str) -> Self::Output {
        match (self.is_empty(), rhs.is_empty()) {
            (true, true) => Oco::Borrowed(""),
            (false, true) => match self {
                Oco::Counted(l) => Oco::Counted(l),
                Oco::Owned(l) => Oco::Owned(l),
                Oco::Borrowed(l) => Oco::Owned(l.to_string()),
            },
            _ => Oco::Owned(String::from(self) + rhs),
        }
    }
}

impl<'b> Add<Cow<'b, str>> for Oco<'_, str> {
    type Output = Oco<'static, str>;

    /// Concatenates `self` and `rhs` into a new `Oco<'static, str>`.
    ///
    /// In the general case this allocates a fresh `String`; the
    /// `'static` lifetime on the output refers to that freshly owned
    /// buffer, not to borrowed `'static` input. Fast-paths that avoid
    /// the allocation:
    ///
    /// - both sides empty → returns `Oco::Borrowed("")`.
    /// - `rhs` empty and `self` is [`Oco::Counted`] or [`Oco::Owned`] →
    ///   returns `self` unchanged.
    /// - `self` empty and `rhs` is [`Cow::Owned`] → returns `rhs`'s
    ///   `String` wrapped in [`Oco::Owned`].
    fn add(self, rhs: Cow<'b, str>) -> Self::Output {
        match (self.is_empty(), rhs.is_empty()) {
            (true, true) => Oco::Borrowed(""),
            (false, true) => match self {
                Oco::Counted(l) => Oco::Counted(l),
                Oco::Owned(l) => Oco::Owned(l),
                Oco::Borrowed(l) => Oco::Owned(l.to_string()),
            },
            (true, false) => match rhs {
                Cow::Owned(r) => Oco::Owned(r),
                Cow::Borrowed(r) => Oco::Owned(r.to_string()),
            },
            (false, false) => Oco::Owned(String::from(self) + rhs.as_ref()),
        }
    }
}

impl<'b> Add<Oco<'b, str>> for Oco<'_, str> {
    type Output = Oco<'static, str>;

    /// Concatenates `self` and `rhs` into a new `Oco<'static, str>`.
    ///
    /// In the general case this allocates a fresh `String`; the
    /// `'static` lifetime on the output refers to that freshly owned
    /// buffer, not to borrowed `'static` input. Fast-paths that avoid
    /// the allocation:
    ///
    /// - both sides empty → returns `Oco::Borrowed("")`.
    /// - `rhs` empty and `self` is [`Oco::Counted`] or [`Oco::Owned`] →
    ///   returns `self` unchanged.
    /// - `self` empty and `rhs` is [`Oco::Counted`] or [`Oco::Owned`] →
    ///   returns `rhs` unchanged.
    fn add(self, rhs: Oco<'b, str>) -> Self::Output {
        match (self.is_empty(), rhs.is_empty()) {
            (true, true) => Oco::Borrowed(""),
            (false, true) => match self {
                Oco::Counted(l) => Oco::Counted(l),
                Oco::Owned(l) => Oco::Owned(l),
                Oco::Borrowed(l) => Oco::Owned(l.to_string()),
            },
            (true, false) => match rhs {
                Oco::Counted(r) => Oco::Counted(r),
                Oco::Owned(r) => Oco::Owned(r),
                Oco::Borrowed(r) => Oco::Owned(r.to_string()),
            },
            (false, false) => Oco::Owned(String::from(self) + rhs.as_ref()),
        }
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

impl<'a, T> Deserialize<'a> for Oco<'static, T>
where
    T: ?Sized + ToOwned + 'a,
    T::Owned: DeserializeOwned,
    Arc<T>: From<T::Owned>,
{
    /// Deserializes into the [`Oco::Counted`] variant so that the first
    /// `.clone()` after deserialization is `O(1)` rather than `O(n)`. The
    /// owned value produced by the deserializer is moved straight into an
    /// `Arc<T>` (zero extra copy of the payload for `str`, `[T]`, and any
    /// sized `T` that satisfies `Arc<T>: From<T::Owned>`).
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        <T::Owned>::deserialize(deserializer)
            .map(|v| Oco::Counted(Arc::from(v)))
    }
}

impl<'a, T> Serialize for Oco<'a, T>
where
    T: ?Sized + ToOwned + 'a,
    for<'b> &'b T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.as_ref().serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_fmt_should_display_quotes_for_strings() {
        let s: Oco<str> = Oco::Borrowed("hello");
        assert_eq!(format!("{s:?}"), "\"hello\"");
        let s: Oco<str> = Oco::Counted(Arc::from("hello"));
        assert_eq!(format!("{s:?}"), "\"hello\"");
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
    fn add_two_empty_should_return_borrowed_empty_without_allocating() {
        let s: Oco<str> = Oco::Borrowed("");
        let result = s + "";
        assert!(result.is_borrowed());
        assert_eq!(result, "");

        let s: Oco<str> = Oco::Borrowed("");
        let result = s + Cow::Borrowed("");
        assert!(result.is_borrowed());

        let s: Oco<str> = Oco::Borrowed("");
        let result = s + Oco::<str>::Borrowed("");
        assert!(result.is_borrowed());
    }

    #[test]
    fn add_empty_rhs_to_counted_should_preserve_arc() {
        let arc: Arc<str> = Arc::from("hello");
        let s: Oco<str> = Oco::Counted(Arc::clone(&arc));
        let result = s + "";
        match result {
            Oco::Counted(rc) => assert!(Arc::ptr_eq(&rc, &arc)),
            other => panic!("expected Counted, got {other:?}"),
        }

        let s: Oco<str> = Oco::Counted(Arc::clone(&arc));
        let result = s + Oco::<str>::Borrowed("");
        match result {
            Oco::Counted(rc) => assert!(Arc::ptr_eq(&rc, &arc)),
            other => panic!("expected Counted, got {other:?}"),
        }
    }

    #[test]
    fn add_empty_rhs_to_owned_should_preserve_buffer() {
        let s: Oco<str> = Oco::Owned("hello".to_string());
        let ptr = s.as_str().as_ptr();
        let result = s + "";
        match result {
            Oco::Owned(string) => {
                assert_eq!(string.as_str().as_ptr(), ptr);
                assert_eq!(string, "hello");
            }
            other => panic!("expected Owned, got {other:?}"),
        }
    }

    #[test]
    fn add_empty_self_to_counted_oco_should_preserve_arc() {
        let arc: Arc<str> = Arc::from("world");
        let s: Oco<str> = Oco::Borrowed("");
        let result = s + Oco::Counted(Arc::clone(&arc));
        match result {
            Oco::Counted(rc) => assert!(Arc::ptr_eq(&rc, &arc)),
            other => panic!("expected Counted, got {other:?}"),
        }
    }

    #[test]
    fn add_empty_self_to_owned_cow_should_preserve_buffer() {
        let owned = String::from("world");
        let ptr = owned.as_str().as_ptr();
        let s: Oco<str> = Oco::Borrowed("");
        let result = s + Cow::Owned(owned);
        match result {
            Oco::Owned(string) => {
                assert_eq!(string.as_str().as_ptr(), ptr);
                assert_eq!(string, "world");
            }
            other => panic!("expected Owned, got {other:?}"),
        }
    }

    #[test]
    fn as_str_should_return_a_str() {
        let s: Oco<str> = Oco::Borrowed("hello");
        assert_eq!(s.as_str(), "hello");
        let s: Oco<str> = Oco::Counted(Arc::from("hello"));
        assert_eq!(s.as_str(), "hello");
    }

    #[test]
    fn as_slice_should_return_a_slice() {
        let s: Oco<[i32]> = Oco::Borrowed([1, 2, 3].as_slice());
        assert_eq!(s.as_slice(), [1, 2, 3].as_slice());
        let s: Oco<[i32]> = Oco::Counted(Arc::from([1, 2, 3]));
        assert_eq!(s.as_slice(), [1, 2, 3].as_slice());
    }

    #[test]
    fn default_for_str_should_return_an_empty_borrowed_string() {
        let s: Oco<str> = Default::default();
        assert!(s.is_empty());
        assert!(
            s.is_borrowed(),
            "default Oco<str> should be Borrowed for zero-alloc cheap clones"
        );
    }

    #[test]
    fn default_for_slice_should_return_an_empty_borrowed_slice() {
        let s: Oco<[i32]> = Default::default();
        assert!(s.is_empty());
        assert!(
            s.is_borrowed(),
            "default Oco<[T]> should be Borrowed for zero-alloc cheap clones"
        );
    }

    #[test]
    fn default_for_c_str_should_return_an_empty_borrowed_c_str() {
        let s: Oco<CStr> = Default::default();
        assert!(s.to_bytes().is_empty());
        assert!(s.is_borrowed());
    }

    #[test]
    fn default_for_os_str_should_return_an_empty_borrowed_os_str() {
        let s: Oco<OsStr> = Default::default();
        assert!(s.is_empty());
        assert!(s.is_borrowed());
    }

    #[test]
    fn default_for_path_should_return_an_empty_borrowed_path() {
        let s: Oco<Path> = Default::default();
        assert_eq!(s.as_os_str(), "");
        assert!(s.is_borrowed());
    }

    #[test]
    fn cloned_owned_string_should_make_counted_str() {
        let s: Oco<str> = Oco::Owned(String::from("hello"));
        assert!(s.clone().is_counted());
    }

    #[test]
    fn cloned_borrowed_str_should_make_borrowed_str() {
        let s: Oco<str> = Oco::Borrowed("hello");
        assert!(s.clone().is_borrowed());
    }

    #[test]
    fn cloned_counted_str_should_make_counted_str() {
        let s: Oco<str> = Oco::Counted(Arc::from("hello"));
        assert!(s.clone().is_counted());
    }

    #[test]
    fn cloned_inplace_owned_string_should_make_counted_str_and_become_counted()
    {
        let mut s: Oco<str> = Oco::Owned(String::from("hello"));
        assert!(s.clone_inplace().is_counted());
        assert!(s.is_counted());
    }

    #[test]
    fn cloned_inplace_borrowed_str_should_make_borrowed_str_and_remain_borrowed()
     {
        let mut s: Oco<str> = Oco::Borrowed("hello");
        assert!(s.clone_inplace().is_borrowed());
        assert!(s.is_borrowed());
    }

    #[test]
    fn cloned_inplace_counted_str_should_make_counted_str_and_remain_counted() {
        let mut s: Oco<str> = Oco::Counted(Arc::from("hello"));
        assert!(s.clone_inplace().is_counted());
        assert!(s.is_counted());
    }

    #[test]
    fn serialization_works() {
        let s = serde_json::to_string(&Oco::Borrowed("foo"))
            .expect("should serialize string");
        assert_eq!(s, "\"foo\"");
    }

    #[test]
    fn deserialization_works() {
        let s: Oco<str> = serde_json::from_str("\"bar\"")
            .expect("should deserialize from string");
        assert_eq!(s, Oco::from(String::from("bar")));
    }

    #[test]
    fn deserialization_produces_counted_variant() {
        let s: Oco<str> = serde_json::from_str("\"hello\"")
            .expect("should deserialize from string");
        assert!(
            s.is_counted(),
            "deserialized Oco should be Counted so first clone is O(1)"
        );
        let s2 = s.clone();
        assert!(s2.is_counted());
        assert!(s.is_counted());
    }

    #[test]
    fn from_box_str_should_produce_owned_consistent_with_string() {
        let boxed: Box<str> = "hello".into();
        let o1: Oco<'_, str> = boxed.into();
        assert!(o1.is_owned());

        let owned: String = "hello".to_owned();
        let o2: Oco<'_, str> = owned.into();
        assert!(o2.is_owned());

        assert_eq!(o1, o2);
    }

    #[test]
    fn from_box_slice_should_produce_owned_consistent_with_vec() {
        let boxed: Box<[i32]> = vec![1, 2, 3].into_boxed_slice();
        let o1: Oco<'_, [i32]> = boxed.into();
        assert!(o1.is_owned());

        let v: Vec<i32> = vec![1, 2, 3];
        let o2: Oco<'_, [i32]> = v.into();
        assert!(o2.is_owned());

        assert_eq!(o1, o2);
    }

    #[test]
    fn deserialization_produces_counted_variant_for_slice() {
        let s: Oco<[i32]> = serde_json::from_str("[1,2,3]")
            .expect("should deserialize from slice");
        assert!(s.is_counted());
        assert_eq!(s.as_slice(), &[1, 2, 3]);
    }

    #[test]
    fn serde_round_trip_lands_on_counted_for_every_input_variant() {
        // All three input variants serialise to the same on-wire form,
        // and deserialisation lands on `Counted`. After a round trip the
        // observed variant is therefore stable: code branching on
        // `is_counted()` keeps working regardless of which variant the
        // value started in.
        for input in [
            Oco::<str>::Borrowed("hello"),
            Oco::<str>::Owned("hello".to_string()),
            Oco::<str>::Counted(Arc::from("hello")),
        ] {
            let wire =
                serde_json::to_string(&input).expect("serialize should work");
            assert_eq!(wire, "\"hello\"");

            let out: Oco<'static, str> =
                serde_json::from_str(&wire).expect("deserialize should work");
            assert_eq!(out, input);
            assert!(
                out.is_counted(),
                "round-trip should always land on Counted"
            );
        }
    }
}
