use crate::{children::ViewFn, AsyncChildren, IntoView};
use leptos_macro::component;
use std::{future::Future, sync::Arc};
use tachys::prelude::FutureViewExt;

/// TODO docs!
#[component]
pub fn Transition<Chil, ChilFn, ChilFut>(
    #[prop(optional, into)] fallback: ViewFn,
    children: AsyncChildren<Chil, ChilFn, ChilFut>,
) -> impl IntoView
where
    Chil: IntoView + 'static,
    ChilFn: Fn() -> ChilFut + Clone + Send + 'static,
    ChilFut: Future<Output = Chil> + Send + 'static,
{
    let children = children.into_inner();
    move || {
        children()
            .suspend()
            .transition()
            .with_fallback(fallback.run())
            .track()
    }
}
