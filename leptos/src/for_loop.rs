use crate::into_view::IntoView;
use leptos_macro::component;
use reactive_graph::owner::Owner;
use std::{hash::Hash, marker::PhantomData};
use tachys::{
    reactive_graph::OwnedView,
    renderer::Renderer,
    view::{keyed::keyed, RenderHtml},
};

/// Iterates over children and displays them, keyed by the `key` function given.
///
/// This is much more efficient than naively iterating over nodes with `.iter().map(|n| view! { ... })...`,
/// as it avoids re-creating DOM nodes that are not being changed.
///
/// ```
/// # use leptos::*;
///
/// #[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// struct Counter {
///   id: usize,
///   count: RwSignal<i32>
/// }
///
/// #[component]
/// fn Counters() -> impl IntoView {
///   let (counters, set_counters) = create_signal::<Vec<Counter>>(vec![]);
///
///   view! {
///     <div>
///       <For
///         // a function that returns the items we're iterating over; a signal is fine
///         each=move || counters.get()
///         // a unique key for each item
///         key=|counter| counter.id
///         // renders each item to a view
///         children=move |counter: Counter| {
///           view! {
///             <button>"Value: " {move || counter.count.get()}</button>
///           }
///         }
///       />
///     </div>
///   }
/// }
/// ```
#[cfg_attr(feature = "tracing", tracing::instrument(level = "trace", skip_all))]
pub fn For<Rndr, IF, I, T, EF, N, KF, K>(
    /// Items over which the component should iterate.
    each: IF,
    /// A key function that will be applied to each item.
    key: KF,
    /// A function that takes the item, and returns the view that will be displayed for each item.
    children: EF,
) -> impl IntoView
where
    IF: Fn() -> I + Send + 'static,
    I: IntoIterator<Item = T> + Send,
    EF: Fn(T) -> N + Send + Clone + 'static,
    N: IntoView + 'static,
    KF: Fn(&T) -> K + Send + Clone + 'static,
    K: Eq + Hash + 'static,
    T: Send + 'static,
{
    // this takes the owner of the For itself
    // this will end up with N + 1 children
    // 1) the effect for the `move || keyed(...)` updates
    // 2) an owner for each child
    //
    // this means
    // a) the reactive owner for each row will not be cleared when the whole list updates
    // b) context provided in each row will not wipe out the others
    /*let parent = Owner::current().expect("no reactive owner");
    let children = move |child| {
        let owner = parent.with(Owner::new);
        let view = owner.with(|| children(child));
        OwnedView::new_with_owner(view, owner)
    };
    move || keyed(each(), key.clone(), children.clone())*/
    "todo"
}

#[component]
pub fn FlatFor<Rndr, IF, I, T, EF, N, KF, K>(
    /// Items over which the component should iterate.
    each: IF,
    /// A key function that will be applied to each item.
    key: KF,
    /// A function that takes the item, and returns the view that will be displayed for each item.
    children: EF,
    #[prop(optional)] _rndr: PhantomData<Rndr>,
) -> impl IntoView
where
    IF: Fn() -> I + 'static,
    I: IntoIterator<Item = T>,
    EF: Fn(T) -> N + Clone + 'static,
    N: RenderHtml<Rndr> + 'static,
    KF: Fn(&T) -> K + Clone + 'static,
    K: Eq + Hash + 'static,
    T: 'static,
    Rndr: Renderer + 'static,
{
    //move || keyed(each(), key.clone(), children.clone())
    "bar"
}

#[cfg(test)]
mod tests {
    use crate::For;
    use leptos_macro::view;
    use reactive_graph::{signal::RwSignal, signal_traits::SignalGet};
    use tachys::{
        html::element::HtmlElement, prelude::ElementChild,
        renderer::mock_dom::MockDom, view::Render,
    };

    #[test]
    fn creates_list() {
        let values = RwSignal::new(vec![1, 2, 3, 4, 5]);
        let list: HtmlElement<_, _, _, MockDom> = view! {
            <ol>
                <For each=move || values.get() key=|i| *i let:i>
                    <li>{i}</li>
                </For>
            </ol>
        };
        let list = list.build();
        assert_eq!(
            list.el.to_debug_html(),
            "<ol><li>1</li><li>2</li><li>3</li><li>4</li><li>5</li></ol>"
        );
    }
}
