use crate::{children::TypedChildrenMut, IntoView};
use any_error::Error;
use leptos_macro::component;
use tachys::view::error_boundary::TryCatchBoundary;

#[component]
pub fn ErrorBoundary<FalFn, Fal, Chil>(
    /// The elements that will be rendered, which may include one or more `Result<_>` types.
    children: TypedChildrenMut<Chil>,
    /// A fallback that will be shown if an error occurs.
    fallback: FalFn,
) -> impl IntoView
where
    FalFn: FnMut(Error) -> Fal + Clone + Send + 'static,
    Fal: IntoView + 'static,
    Chil: IntoView + 'static,
{
    let mut children = children.into_inner();
    // TODO dev-mode warning about Suspense/ErrorBoundary ordering
    move || children().catch(fallback.clone())
}
