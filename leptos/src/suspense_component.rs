use crate::{
    children::{ToChildren, TypedChildrenFn, TypedChildrenMut, ViewFn},
    IntoView,
};
use leptos_macro::component;
use leptos_reactive::untrack;
use std::{future::Future, sync::Arc};
use tachys::{async_views::SuspenseBoundary, prelude::FutureViewExt};

/// An async, typed equivalent to [`Children`], which takes a generic but preserves
/// type information to allow the compiler to optimize the view more effectively.
pub struct AsyncChildren<T, F, Fut>(pub(crate) F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = T>;

impl<T, F, Fut> AsyncChildren<T, F, Fut>
where
    F: Fn() -> Fut,
    Fut: Future<Output = T>,
{
    pub fn into_inner(self) -> F {
        self.0
    }
}

impl<T, F, Fut> ToChildren<F> for AsyncChildren<T, F, Fut>
where
    F: Fn() -> Fut,
    Fut: Future<Output = T>,
{
    fn to_children(f: F) -> Self {
        AsyncChildren(f)
    }
}

/// TODO docs!
#[component]
pub fn Suspense<Chil>(
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
        crate::logging::log!("running innner thing");
        untrack(|| {
            SuspenseBoundary::<false, _, _>::new(
                fallback.clone(),
                (children.clone())(),
            )
        })
        // TODO track
    }
}
