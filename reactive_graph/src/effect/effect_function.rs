/// Trait to enable effect functions that have zero or one parameter
pub trait EffectFunction<T, M> {
    /// Call this to execute the function. In case the actual function has no parameters
    /// the parameter `p` will simply be ignored.
    fn run(&mut self, p: Option<T>) -> T;
}

/// Marker for single parameter functions
pub struct SingleParam;
/// Marker for no parameter functions
pub struct NoParam;

impl<Func, T> EffectFunction<T, SingleParam> for Func
where
    Func: FnMut(Option<T>) -> T,
{
    #[inline(always)]
    fn run(&mut self, p: Option<T>) -> T {
        (self)(p)
    }
}

impl<Func> EffectFunction<(), NoParam> for Func
where
    Func: FnMut(),
{
    #[inline(always)]
    fn run(&mut self, _: Option<()>) {
        self()
    }
}
