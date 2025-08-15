use crate::{
    children::{TypedChildren, ViewFnOnce},
    error::ErrorBoundarySuspendedChildren,
    suspense_component::SuspenseBoundary,
    IntoView,
};
use leptos_macro::component;
use reactive_graph::{
    computed::{suspense::SuspenseContext, ArcMemo},
    effect::Effect,
    owner::{provide_context, use_context, Owner},
    signal::ArcRwSignal,
    traits::{Get, Set, Track, With},
    wrappers::write::SignalSetter,
};
use slotmap::{DefaultKey, SlotMap};
use tachys::reactive_graph::OwnedView;

/// If any [`Resource`](leptos_reactive::Resource) is read in the `children` of this
/// component, it will show the `fallback` while they are loading. Once all are resolved,
/// it will render the `children`.
///
/// Unlike [`Suspense`](crate::Suspense), this will not fall
/// back to the `fallback` state if there are further changes after the initial load.
///
/// Note that the `children` will be rendered initially (in order to capture the fact that
/// those resources are read under the suspense), so you cannot assume that resources read
/// synchronously have
/// `Some` value in `children`. However, you can read resources asynchronously by using
/// [Suspend](crate::prelude::Suspend).
///
/// ```
/// # use leptos::prelude::*;
/// # if false { // don't run in doctests
/// async fn fetch_cats(how_many: u32) -> Vec<String> { vec![] }
///
/// let (cat_count, set_cat_count) = signal::<u32>(1);
///
/// let cats = Resource::new(move || cat_count.get(), |count| fetch_cats(count));
///
/// view! {
///   <div>
///     <Transition fallback=move || view! { <p>"Loading (Suspense Fallback)..."</p> }>
///       // you can access a resource synchronously
///       {move || {
///           cats.get().map(|data| {
///             data
///               .into_iter()
///               .map(|src| {
///                   view! {
///                     <img src={src}/>
///                   }
///               })
///               .collect_view()
///           })
///         }
///       }
///       // or you can use `Suspend` to read resources asynchronously
///       {move || Suspend::new(async move {
///         cats.await
///               .into_iter()
///               .map(|src| {
///                   view! {
///                     <img src={src}/>
///                   }
///               })
///               .collect_view()
///       })}
///     </Transition>
///   </div>
/// }
/// # ;}
/// ```
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
    let error_boundary_parent = use_context::<ErrorBoundarySuspendedChildren>();

    let owner = Owner::new();
    owner.with(|| {
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
            error_boundary_parent,
        })
    })
}
