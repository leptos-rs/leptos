use crate::{
    children::{TypedChildren, ViewFnOnce},
    suspense_component::SuspenseBoundary,
    IntoView,
};
use leptos_macro::component;
use reactive_graph::{
    computed::{suspense::SuspenseContext, ArcMemo},
    effect::Effect,
    owner::{provide_context, Owner},
    signal::ArcRwSignal,
    traits::{Get, Set, Track, With},
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
    let (starts_local, id) = {
        Owner::current_shared_context()
            .map(|sc| {
                let id = sc.next_id();
                (sc.get_incomplete_chunk(&id), id)
            })
            .unwrap_or_else(|| (false, Default::default()))
    };
    let fallback = fallback.run();
    let children = children.into_inner()();
    let tasks = ArcRwSignal::new(SlotMap::<DefaultKey, ()>::new());
    provide_context(SuspenseContext {
        tasks: tasks.clone(),
    });
    let none_pending = ArcMemo::new(move |prev: Option<&bool>| {
        tasks.track();
        if prev.is_none() && starts_local {
            false
        } else {
            tasks.with(SlotMap::is_empty)
        }
    });
    if let Some(set_pending) = set_pending {
        Effect::new_isomorphic({
            let none_pending = none_pending.clone();
            move |_| {
                set_pending.set(!none_pending.get());
            }
        });
    }

    OwnedView::new(SuspenseBoundary::<true, _, _> {
        id,
        none_pending,
        fallback,
        children,
    })
}
