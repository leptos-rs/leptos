use leptos_dom::{IntoView, View};
use leptos_reactive::{MaybeSignal, SignalGet};

/// New-type wrapper for the a function that returns a view with `From` and `Default` traits implemented
/// to enable optional props in for example `<Show>` and `<Suspense>`.
#[derive(Clone)]
pub struct ViewFn(MaybeSignal<View>);

impl Default for ViewFn {
    fn default() -> Self {
        Self(MaybeSignal::Static(().into_view()))
    }
}

impl<F, IV> From<F> for ViewFn
where
    F: crate::Invocable<Value = IV> + 'static,
    IV: IntoView + Clone + 'static,
{
    fn from(value: F) -> Self {
        Self(MaybeSignal::derive(move || value.invoke().into_view()))
    }
}

impl ViewFn {
    /// Execute the wrapped function
    pub fn run(&self) -> View {
        self.0.get()
    }
}
