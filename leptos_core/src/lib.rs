#[cfg(any(feature = "csr", feature = "hydrate", feature = "ssr"))]
mod for_component;
#[cfg(any(feature = "csr", feature = "hydrate", feature = "ssr"))]
mod map;
#[cfg(any(feature = "csr", feature = "hydrate", feature = "ssr"))]
mod suspense;

#[cfg(any(feature = "csr", feature = "hydrate", feature = "ssr"))]
pub use for_component::*;
#[cfg(any(feature = "csr", feature = "hydrate", feature = "ssr"))]
pub use map::*;
#[cfg(any(feature = "csr", feature = "hydrate", feature = "ssr"))]
pub use suspense::*;

pub trait Prop {
    type Builder;

    /// The builder should be automatically generated using the `Prop` derive macro.
    fn builder() -> Self::Builder;
}
