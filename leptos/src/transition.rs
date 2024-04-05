use crate::{
    children::{TypedChildrenFn, ViewFn},
    IntoView,
};
use leptos_macro::component;
use tachys::async_views::SuspenseBoundary;

/// TODO docs!
#[component]
pub fn Transition<Chil>(
    #[prop(optional, into)] fallback: ViewFn,
    children: TypedChildrenFn<Chil>,
) -> impl IntoView
where
    Chil: IntoView + 'static,
{
    let children = children.into_inner();
    let fallback = move || fallback.clone().run();
    // TODO check this against islands
    move || {
        SuspenseBoundary::<true, _, _>::new(
            fallback.clone(),
            (children.clone())(),
        )
        // TODO track
    }
}
