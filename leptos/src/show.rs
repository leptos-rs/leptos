use crate::{
    children::{TypedChildrenFn, ViewFn},
    IntoView,
};
use leptos_macro::component;
use reactive_graph::{
    computed::ArcMemo,
    traits::{GetUntracked, Track},
    untrack,
};
use tachys::either::Either;

#[component]
pub fn Show<W, C>(
    /// The children will be shown whenever the condition in the `when` closure returns `true`.
    children: TypedChildrenFn<C>,
    /// A closure that returns a bool that determines whether this thing runs
    when: W,
    /// A closure that returns what gets rendered if the when statement is false. By default this is the empty view.
    #[prop(optional, into)]
    fallback: ViewFn,
) -> impl IntoView
where
    W: Fn() -> bool + Send + Sync + 'static,
    C: IntoView + 'static,
{
    let memoized_when = ArcMemo::new(move |_| when());
    let children = children.into_inner();

    move || {
        memoized_when.track();
        untrack(|| match memoized_when.get_untracked() {
            true => Either::Left(children()),
            false => Either::Right(fallback.run()),
        })
    }
}
