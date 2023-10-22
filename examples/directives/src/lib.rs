use leptos::{ev::click, html::AnyElement, *};

pub fn highlight(el: HtmlElement<AnyElement>) {
    let mut highlighted = false;

    let _ = el.clone().on(click, move |_| {
        highlighted = !highlighted;

        if highlighted {
            let _ = el.clone().style("background-color", "yellow");
        } else {
            let _ = el.clone().style("background-color", "transparent");
        }
    });
}

pub fn copy_to_clipboard(el: HtmlElement<AnyElement>, content: &str) {
    let content = content.to_string();

    let _ = el.clone().on(click, move |evt| {
        evt.prevent_default();
        evt.stop_propagation();

        let _ = window()
            .navigator()
            .clipboard()
            .expect("navigator.clipboard to be available")
            .write_text(&content);

        let _ = el.clone().inner_html(format!("Copied \"{}\"", &content));
    });
}

#[component]
pub fn SomeComponent() -> impl IntoView {
    view! {
        <p>Some paragraphs</p>
        <p>that can be clicked</p>
        <p>in order to highlight them</p>
    }
}

#[component]
pub fn App() -> impl IntoView {
    let data = "Hello World!";

    view! {
        <a href="#" use:copy_to_clipboard=data>"Copy \"" {data} "\" to clipboard"</a>
        <SomeComponent use:highlight />
    }
}
