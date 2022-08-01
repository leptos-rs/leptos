mod for_component;
mod map;

pub use for_component::*;

pub trait Prop {
    type Builder;

    /// The builder should be automatically generated using the `Prop` derive macro.
    fn builder() -> Self::Builder;
}
