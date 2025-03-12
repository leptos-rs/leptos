//! Side effects that run in response to changes in the reactive values they read from.

#[allow(clippy::module_inception)]
mod effect;
mod effect_function;
mod immediate;
mod inner;
mod render_effect;

pub use effect::*;
pub use effect_function::*;
pub use immediate::*;
pub use render_effect::*;

/// Creates a new render effect, which immediately runs `fun`.
#[inline(always)]
#[track_caller]
#[deprecated = "This function is being removed to conform to Rust idioms. \
                Please use `RenderEffect::new()` instead."]
pub fn create_render_effect<T>(
    fun: impl FnMut(Option<T>) -> T + 'static,
) -> RenderEffect<T>
where
    T: 'static,
{
    RenderEffect::new(fun)
}
