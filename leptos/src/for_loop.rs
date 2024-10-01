use crate::into_view::IntoView;
use leptos_macro::component;
use reactive_graph::{
    owner::Owner,
    signal::{ArcRwSignal, ReadSignal},
    traits::Set,
};
use std::hash::Hash;
use tachys::{reactive_graph::OwnedView, view::keyed::keyed};

/// Iterates over children and displays them, keyed by the `key` function given.
///
/// This is much more efficient than naively iterating over nodes with `.iter().map(|n| view! { ... })...`,
/// as it avoids re-creating DOM nodes that are not being changed.
///
/// ```
/// # use leptos::prelude::*;
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
#[component]
pub fn For<IF, I, T, EF, N, KF, K>(
    /// Items over which the component should iterate.
    each: IF,
    /// A key function that will be applied to each item.
    key: KF,
    /// A function that takes the item, and returns the view that will be displayed for each item.
    children: EF,
) -> impl IntoView
where
    IF: Fn() -> I + Send + 'static,
    I: IntoIterator<Item = T> + Send + 'static,
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
    let parent = Owner::current().expect("no reactive owner");
    let children = move |_, child| {
        let owner = parent.with(Owner::new);
        let view = owner.with(|| children(child));
        (|_| {}, OwnedView::new_with_owner(view, owner))
    };
    move || keyed(each(), key.clone(), children.clone())
}

/// Iterates over children and displays them, keyed by the `key` function given.
///
/// Compared with For, it has an additional index parameter, which can be used to obtain the current index in real time.
///
/// This is much more efficient than naively iterating over nodes with `.iter().map(|n| view! { ... })...`,
/// as it avoids re-creating DOM nodes that are not being changed.
///
/// ```
/// # use leptos::prelude::*;
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
///       <ForEnumerate
///         // a function that returns the items we're iterating over; a signal is fine
///         each=move || counters.get()
///         // a unique key for each item
///         key=|counter| counter.id
///         // renders each item to a view
///         children={move |index: ReadSignal<usize>, counter: Counter| {
///           view! {
///             <button>{move || index.get()} ". Value: " {move || counter.count.get()}</button>
///           }
///         }}
///       />
///     </div>
///   }
/// }
/// ```
#[cfg_attr(feature = "tracing", tracing::instrument(level = "trace", skip_all))]
#[component]
pub fn ForEnumerate<IF, I, T, EF, N, KF, K>(
    /// Items over which the component should iterate.
    each: IF,
    /// A key function that will be applied to each item.
    key: KF,
    /// A function that takes the index and the item, and returns the view that will be displayed for each item.
    children: EF,
) -> impl IntoView
where
    IF: Fn() -> I + Send + 'static,
    I: IntoIterator<Item = T> + Send + 'static,
    EF: Fn(ReadSignal<usize>, T) -> N + Send + Clone + 'static,
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
    let parent = Owner::current().expect("no reactive owner");
    let children = move |index, child| {
        let owner = parent.with(Owner::new);
        let (index, set_index) = ArcRwSignal::new(index).split();
        let view = owner.with(|| children(index.into(), child));
        (
            move |index| set_index.set(index),
            OwnedView::new_with_owner(view, owner),
        )
    };
    move || keyed(each(), key.clone(), children.clone())
}
/*
#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use leptos_macro::view;
    use tachys::{html::element::HtmlElement, prelude::ElementChild};

    #[test]
    fn creates_list() {
        Owner::new().with(|| {
            let values = RwSignal::new(vec![1, 2, 3, 4, 5]);
            let list: View<HtmlElement<_, _, _>> = view! {
                <ol>
                    <For each=move || values.get() key=|i| *i let:i>
                        <li>{i}</li>
                    </For>
                </ol>
            };
            assert_eq!(
                list.to_html(),
                "<ol><li>1</li><li>2</li><li>3</li><li>4</li><li>5</li><!></\
                 ol>"
            );
        });
    }

    #[test]
    fn creates_list_enumerate() {
        Owner::new().with(|| {
            let values = RwSignal::new(vec![1, 2, 3, 4, 5]);
            let list: View<HtmlElement<_, _, _>> = view! {
                <ol>
                    <ForEnumerate each=move || values.get() key=|i| *i let(index, i)>
                        <li>{move || index.get()}"-"{i}</li>
                    </ForEnumerate>
                </ol>
            };
            assert_eq!(
                list.to_html(),
                "<ol><li>0<!>-<!>1</li><li>1<!>-<!>2</li><li>2<!>-<!>3</li><li>3\
                <!>-<!>4</li><li>4<!>-<!>5</li><!></ol>"
            );

            let list: View<HtmlElement<_, _, _>> = view! {
                <ol>
                    <ForEnumerate each=move || values.get() key=|i| *i let(index, i)>
                        <li>{move || index.get()}"-"{i}</li>
                    </ForEnumerate>
                </ol>
            };
            values.set(vec![5, 4, 1, 2, 3]);
            assert_eq!(
                list.to_html(),
                "<ol><li>0<!>-<!>5</li><li>1<!>-<!>4</li><li>2<!>-<!>1</li><li>3\
                <!>-<!>2</li><li>4<!>-<!>3</li><!></ol>"
            );
        });
    }
}
 */
