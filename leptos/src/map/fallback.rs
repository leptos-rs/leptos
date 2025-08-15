use std::sync::Arc;

use tachys::prelude::{AnyView, IntoAny};

use crate::IntoView;

/// Wrapper for a function that takes a parameter and returns a view.
pub struct ViewFnWithParam<P = ()>(
    Arc<dyn Fn(P) -> AnyView + Send + Sync + 'static>,
);

impl<P> Clone for ViewFnWithParam<P> {
    fn clone(&self) -> Self {
        ViewFnWithParam(Arc::clone(&self.0))
    }
}

impl<P> ViewFnWithParam<P> {
    /// Runs the function with the given parameter.
    pub fn run(&self, param: P) -> AnyView {
        (self.0)(param)
    }
}

impl<P, F, V> From<F> for ViewFnWithParam<P>
where
    F: Fn(P) -> V + Send + Sync + 'static,
    V: IntoView,
{
    fn from(f: F) -> Self {
        ViewFnWithParam(Arc::new(move |param| f(param).into_any()))
    }
}

impl<E> Default for ViewFnWithParam<E> {
    fn default() -> Self {
        ViewFnWithParam::<E>(Arc::new(|_| ().into_any()))
    }
}
