use leptos_macro::component;
use reactive_graph::owner::Owner;
use std::{hash::Hash, marker::PhantomData};
use tachys::{
    reactive_graph::OwnedView,
    renderer::Renderer,
    view::{keyed::keyed, RenderHtml},
};

#[component]
pub fn For<Rndr, IF, I, T, EF, N, KF, K>(
    /// Items over which the component should iterate.
    each: IF,
    /// A key function that will be applied to each item.
    key: KF,
    /// A function that takes the item, and returns the view that will be displayed for each item.
    children: EF,
    #[prop(optional)] _rndr: PhantomData<Rndr>,
) -> impl RenderHtml<Rndr>
where
    IF: Fn() -> I + 'static,
    I: IntoIterator<Item = T>,
    EF: Fn(T) -> N + Clone + 'static,
    N: RenderHtml<Rndr> + 'static,
    KF: Fn(&T) -> K + Clone + 'static,
    K: Eq + Hash + 'static,
    T: 'static,
    Rndr: Renderer + 'static,
    Rndr::Node: Clone,
    Rndr::Element: Clone,
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
    let children = move |child| {
        let owner = parent.with(Owner::new);
        let view = owner.with(|| children(child));
        OwnedView::new_with_owner(view, owner)
    };
    move || keyed(each(), key.clone(), children.clone())
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
) -> impl RenderHtml<Rndr>
where
    IF: Fn() -> I + 'static,
    I: IntoIterator<Item = T>,
    EF: Fn(T) -> N + Clone + 'static,
    N: RenderHtml<Rndr> + 'static,
    KF: Fn(&T) -> K + Clone + 'static,
    K: Eq + Hash + 'static,
    T: 'static,
    Rndr: Renderer + 'static,
    Rndr::Node: Clone,
    Rndr::Element: Clone,
{
    move || keyed(each(), key.clone(), children.clone())
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
                <For
                    each=move || values.get()
                    key=|i| *i
                    let:i
                >
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
