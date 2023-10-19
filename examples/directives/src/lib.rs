use leptos::{
    ev::{click, mouseenter, mouseleave},
    html::AnyElement,
    logging::log,
    *,
};

pub fn hoverable(el: HtmlElement<AnyElement>) {
    el.on(mouseenter, |_| {
        log!("hovered");
    })
    .on(mouseleave, |_| {
        log!("unhovered");
    });
}

pub fn copy_to_clipboard(el: HtmlElement<AnyElement>, content: &str) {
    let content = content.to_string();

    el.clone().on(click, move |evt| {
        evt.prevent_default();
        evt.stop_propagation();

        let _ = window()
            .navigator()
            .clipboard()
            .expect("navigator.clipboard to be available")
            .write_text(&content);

        el.clone().inner_html(format!("Copied \"{}\"", &content));
    });
}

#[component]
pub fn SomeComponent() -> impl IntoView {
    view! {
        <p>Some paragraphs</p>
        <p>that can be hovered</p>
        <p>Check the dev tools console</p>
    }
}

#[component]
pub fn App() -> impl IntoView {
    let data = "Hello World!";

    view! {
        <a href="#" use:copy_to_clipboard=data>"Copy \"" {data} "\" to clipboard"</a>
        <SomeComponent use:hoverable />
    }
}
