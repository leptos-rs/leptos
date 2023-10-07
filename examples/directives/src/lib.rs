use leptos::{html::AnyElement, *};

pub fn hoverable(el: HtmlElement<AnyElement>) {
    todo!()
}

#[derive(Clone)]
pub struct HighlightableOptions {
    pub highlight_color: String,
    pub idle_color: String,
}

pub fn highlightable(
    el: HtmlElement<AnyElement>,
    options: HighlightableOptions,
) {
    todo!()
}

#[component]
pub fn App() -> impl IntoView {
    view! {}
}
