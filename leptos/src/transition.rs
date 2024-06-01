use crate::{
    children::{TypedChildren, ViewFnOnce},
    suspense_component::SuspenseBoundary,
    IntoView,
};
use leptos_macro::component;
use reactive_graph::{
    computed::{suspense::SuspenseContext, ArcMemo},
    owner::provide_context,
    signal::ArcRwSignal,
    traits::With,
};
use slotmap::{DefaultKey, SlotMap};
use tachys::reactive_graph::OwnedView;

/// TODO docs!
#[component]
pub fn Transition<Chil>(
    #[prop(optional, into)] fallback: ViewFnOnce,
    children: TypedChildren<Chil>,
) -> impl IntoView
where
    Chil: IntoView + Send + 'static,
{
    let fallback = fallback.run();
    let children = children.into_inner()();
    let tasks = ArcRwSignal::new(SlotMap::<DefaultKey, ()>::new());
    provide_context(SuspenseContext {
        tasks: tasks.clone(),
    });
    let none_pending = ArcMemo::new(move |_| tasks.with(SlotMap::is_empty));

    OwnedView::new(SuspenseBoundary::<true, _, _> {
        none_pending,
        fallback,
        children,
    })
}
