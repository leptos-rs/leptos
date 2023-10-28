use leptos_dom::{IntoView, View};
use std::rc::Rc;

/// New-type wrapper for the a function that returns a view with `From` and `Default` traits implemented
/// to enable optional props in for example `<Show>` and `<Suspense>`.
#[derive(Clone)]
pub struct ViewFn(Rc<dyn Fn() -> View>);

impl Default for ViewFn {
    fn default() -> Self {
        Self(Rc::new(|| ().into_view()))
    }
}

impl<F, IV> From<F> for ViewFn
where
    F: Fn() -> IV + 'static,
    IV: IntoView,
{
    fn from(value: F) -> Self {
        Self(Rc::new(move || value().into_view()))
    }
}

impl ViewFn {
    /// Execute the wrapped function
    pub fn run(&self) -> View {
        (self.0)()
    }
}
