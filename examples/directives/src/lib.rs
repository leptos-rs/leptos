use leptos::{ev::click, prelude::*};
use web_sys::Element;

// no extra parameter
pub fn highlight(el: Element) {
    let mut highlighted = false;

    let handle = el.clone().on(click, move |_| {
        highlighted = !highlighted;

        if highlighted {
            el.style(("background-color", "yellow"));
        } else {
            el.style(("background-color", "transparent"));
        }
    });
    on_cleanup(move || drop(handle));
}

// one extra parameter
pub fn copy_to_clipboard(el: Element, content: &str) {
    let content = content.to_owned();
    let handle = el.clone().on(click, move |evt| {
        evt.prevent_default();
        evt.stop_propagation();

        let _ = window().navigator().clipboard().write_text(&content);

        el.set_inner_html(&format!("Copied \"{}\"", &content));
    });
    on_cleanup(move || drop(handle));
}

// custom parameter

#[derive(Clone)]
pub struct Amount(usize);

impl From<usize> for Amount {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

// a 'default' value if no value is passed in
impl From<()> for Amount {
    fn from(_: ()) -> Self {
        Self(1)
    }
}

pub fn add_dot(el: Element, amount: Amount) {
    use leptos::wasm_bindgen::JsCast;
    let el = el.unchecked_into::<web_sys::HtmlElement>();

    let handle = el.clone().on(click, move |_| {
        el.set_inner_text(&format!(
            "{}{}",
            el.inner_text(),
            ".".repeat(amount.0)
        ))
    });
    on_cleanup(move || drop(handle));
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
        <a href="#" use:copy_to_clipboard=data>
            "Copy \""
            {data}
            "\" to clipboard"
        </a>
        // automatically applies the directive to every root element in `SomeComponent`
        <SomeComponent use:highlight/>
        // no value will default to `().into()`
        <button use:add_dot>"Add a dot"</button>
        // can manually call `.into()` to convert to the correct type
        // (automatically calling `.into()` prevents using generics in directive functions)
        <button use:add_dot=5.into()>"Add 5 dots"</button>
    }
}
