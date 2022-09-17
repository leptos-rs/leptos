#![feature(auto_traits)]
#![feature(negative_impls)]

#[cfg(any(feature = "csr", feature = "hydrate", feature = "ssr"))]
mod for_component;
mod map;
#[cfg(any(feature = "csr", feature = "hydrate", feature = "ssr"))]
mod suspense;

#[cfg(any(feature = "csr", feature = "hydrate", feature = "ssr"))]
pub use for_component::*;
#[cfg(any(feature = "csr", feature = "hydrate", feature = "ssr"))]
pub use suspense::*;

pub trait Prop {
    type Builder;

    /// The builder should be automatically generated using the `Prop` derive macro.
    fn builder() -> Self::Builder;
}

pub auto trait NotVec {}

impl<T> !NotVec for Vec<T> {}

pub trait IntoVec<T> {
    fn into_vec(self) -> Vec<T>;
}

impl<T> IntoVec<T> for T
where
    T: NotVec,
{
    fn into_vec(self) -> Vec<T> {
        vec![self]
    }
}

impl<T> IntoVec<T> for Vec<T> {
    fn into_vec(self) -> Vec<T> {
        self
    }
}
