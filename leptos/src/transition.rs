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
    let mut fallback = Some(fallback.run());
    // TODO check this against islands
    move || {
        crate::logging::log!("running suspense again");
        SuspenseBoundary::<true, _, _>::new(
            fallback.take(),
            (children.clone())(),
        )
        // TODO track
    }
}
