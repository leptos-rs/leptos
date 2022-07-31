mod for_component;
mod map;
mod show;

pub use for_component::*;
pub use show::*;

pub trait Prop {
    type Builder;

    /// The builder should be automatically generated using the `Prop` derive macro.
    fn builder() -> Self::Builder;
}
