use leptos::{
    ev::{click, mouseenter, mouseleave},
    html::AnyElement,
    logging::log,
    *,
};

pub fn hoverable(el: HtmlElement<AnyElement>) {
    // TODO : remove?
    el.on(mouseenter, |_| {
        log!("hovered");
    })
    .on(mouseleave, |_| {
        log!("unhovered");
    });
}

pub fn copy_to_clipboard(el: HtmlElement<AnyElement>, content: &str) {
    let content = content.to_string();

    el.on(click, move |evt| {
        evt.prevent_default();
        evt.stop_propagation();

        let _ = window()
            .navigator()
            .clipboard()
            .expect("navigator.clipboard to be available")
            .write_text(&content);
    });
}

#[component]
pub fn App() -> impl IntoView {
    view! {
        <a href="#" use:copy_to_clipboard="Hello World!">"Copy to clipboard"</a>
        <div use:hoverable>Hover me and check console</div>
    }
}
