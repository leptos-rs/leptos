use crate::{
    children::{TypedChildren, ViewFnOnce},
    into_view::View,
    suspense_component::{SuspenseBoundary, SuspenseContext},
    IntoView,
};
use leptos_macro::component;
use reactive_graph::{
    computed::ArcMemo, owner::provide_context, signal::ArcRwSignal,
    traits::With,
};
use slotmap::{DefaultKey, SlotMap};
use std::future::Future;
use tachys::{
    renderer::dom::Dom,
    view::{any_view::AnyView, RenderHtml},
};

/// TODO docs!
#[component]
pub fn Transition<Chil>(
    #[prop(optional, into)] fallback: ViewFnOnce,
    children: TypedChildren<Chil>,
) -> impl IntoView
where
    SuspenseBoundary<true, AnyView<Dom>, View<Chil>>: IntoView,
{
    let fallback = fallback.run();
    let children = children.into_inner()();
    let tasks = ArcRwSignal::new(SlotMap::<DefaultKey, ()>::new());
    provide_context(SuspenseContext {
        tasks: tasks.clone(),
    });
    let none_pending = ArcMemo::new(move |_| tasks.with(SlotMap::is_empty));
    SuspenseBoundary::<true, _, _> {
        none_pending,
        fallback,
        children,
    }
}
