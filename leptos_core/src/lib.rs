mod for_component;
mod map;
mod suspense;

pub use for_component::*;
pub use suspense::*;

pub trait Prop {
    type Builder;

    /// The builder should be automatically generated using the `Prop` derive macro.
    fn builder() -> Self::Builder;
}
