use crate::{
    children::{TypedChildren, ViewFnOnce},
    suspense_component::SuspenseBoundary,
    IntoView,
};
use leptos_macro::component;
use reactive_graph::{
    computed::{suspense::SuspenseContext, ArcMemo},
    effect::Effect,
    owner::provide_context,
    signal::ArcRwSignal,
    traits::{Get, With},
    wrappers::write::SignalSetter,
};
use slotmap::{DefaultKey, SlotMap};
use tachys::reactive_graph::OwnedView;

/// TODO docs!
#[component]
pub fn Transition<Chil>(
    /// Will be displayed while resources are pending. By default this is the empty view.
    #[prop(optional, into)]
    fallback: ViewFnOnce,
    /// A function that will be called when the component transitions into or out of
    /// the `pending` state, with its argument indicating whether it is pending (`true`)
    /// or not pending (`false`).
    #[prop(optional, into)]
    set_pending: Option<SignalSetter<bool>>,
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
    if let Some(set_pending) = set_pending {
        Effect::new_isomorphic({
            let none_pending = none_pending.clone();
            move |_| {
                set_pending.set(!none_pending.get());
            }
        });
    }

    OwnedView::new(SuspenseBoundary::<true, _, _> {
        none_pending,
        fallback,
        children,
    })
}
