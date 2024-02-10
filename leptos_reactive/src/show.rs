use crate::children::{ChildrenFn, ViewFn};
use tachy_maccy::component;
use tachy_reaccy::{memo::ArcMemo, signal_traits::SignalGet};
use tachydom::{
    renderer::dom::Dom,
    view::{either::Either, RenderHtml},
};

#[component]
pub fn Show<W>(
    /// The children will be shown whenever the condition in the `when` closure returns `true`.
    children: ChildrenFn,
    /// A closure that returns a bool that determines whether this thing runs
    when: W,
    /// A closure that returns what gets rendered if the when statement is false. By default this is the empty view.
    #[prop(optional, into)]
    fallback: ViewFn,
) -> impl RenderHtml<Dom>
where
    W: Fn() -> bool + Send + Sync + 'static,
{
    let memoized_when = ArcMemo::new(move |_| when());

    move || match memoized_when.get() {
        true => Either::Left(children()),
        false => Either::Right(fallback.run()),
    }
}
