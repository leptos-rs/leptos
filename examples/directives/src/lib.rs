use leptos::{ev::click, html::a, prelude::*};
use web_sys::{Element, HtmlElement};

// no extra parameter
pub fn highlight(el: Element, param: ()) {
    // TODO
    /*
    let mut highlighted = false;

    let _ = el.clone().on(click, move |_| {
        highlighted = !highlighted;

        if highlighted {
            let _ = el.clone().style("background-color", "yellow");
        } else {
            let _ = el.clone().style("background-color", "transparent");
        }
    });
    */
}

// one extra parameter
pub fn copy_to_clipboard(el: Element, content: &'static str) {
    // TODO
    let content = content.to_string();
    leptos::logging::log!("running copy_to_clipboard directive");

    /*let _ = el.clone().on(click, move |evt| {
        evt.prevent_default();
        evt.stop_propagation();

        let _ = window()
            .navigator()
            .clipboard()
            .expect("navigator.clipboard to be available")
            .write_text(&content);

        let _ = el.clone().inner_html(format!("Copied \"{}\"", &content));
    });*/
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

// .into() will automatically be called on the parameter
pub fn add_dot(el: Element, amount: Amount) {
    // TODO
    /*
    _ = el.clone().on(click, move |_| {
        el.set_inner_text(&format!(
            "{}{}",
            el.inner_text(),
            ".".repeat(amount.0)
        ))
    })
    */
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
        // automatically applies the directive to every root element in `SomeComponent`
        //<SomeComponent use:highlight />
        // no value will default to `().into()`
        <button use:add_dot>"Add a dot"</button>
        // `5.into()` automatically called
        <button use:add_dot=5>"Add 5 dots"</button>
    }
}
