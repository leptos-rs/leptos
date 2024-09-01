use crate::{children::TypedChildren, component, IntoView};
use reactive_graph::owner::{provide_context, Owner};
use tachys::reactive_graph::OwnedView;

#[component]
/// Uses the context API to [`provide_context`] to its children and descendants,
/// without overwriting any contexts of the same type in its own reactive scope.
///
/// This prevents issues related to “context shadowing.”
///
/// ```rust
/// use leptos::{context::Provider, prelude::*};
///
/// #[component]
/// pub fn App() -> impl IntoView {
///     // each Provider will only provide the value to its children
///     view! {
///         <Provider value=1u8>
///             // correctly gets 1 from context
///             {use_context::<u8>().unwrap_or(0)}
///         </Provider>
///         <Provider value=2u8>
///             // correctly gets 2 from context
///             {use_context::<u8>().unwrap_or(0)}
///         </Provider>
///         // does not find any u8 in context
///         {use_context::<u8>().unwrap_or(0)}
///     }
/// }
/// ```
pub fn Provider<T, Chil>(
    /// The value to be provided via context.
    value: T,
    children: TypedChildren<Chil>,
) -> impl IntoView
where
    T: Send + Sync + 'static,
    Chil: IntoView + 'static,
{
    let owner = Owner::current()
        .expect("no current reactive Owner found")
        .child();
    let children = children.into_inner();
    let children = owner.with(|| {
        provide_context(value);
        children()
    });
    OwnedView::new_with_owner(children, owner)
}
