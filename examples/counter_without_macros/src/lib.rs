use leptos::{ev, html::*, *};

/// A simple counter view.
// A component is really just a function call: it runs once to create the DOM and reactive system
pub fn counter(cx: Scope, initial_value: i32, step: i32) -> impl IntoView {
    let (value, set_value) = create_signal(cx, initial_value);

    // elements are created by calling a function with a Scope argument
    // the function name is the same as the HTML tag name
    div(cx)
        // children can be added with .child()
        // this takes any type that implements IntoView as its argument
        // for example, a string or an HtmlElement<_>
        .child(
            button(cx)
                // typed events found in leptos::ev
                // 1) prevent typos in event names
                // 2) allow for correct type inference in callbacks
                .on(ev::click, move |_| set_value.update(|value| *value = 0))
                .child("Clear"),
        )
        .child(
            button(cx)
                .on(ev::click, move |_| {
                    set_value.update(|value| *value -= step)
                })
                .child("-1"),
        )
        .child(
            span(cx)
                .child("Value: ")
                // reactive values are passed to .child() as a tuple
                // (Scope, [child function]) so an effect can be created
                .child((cx, move || value.get()))
                .child("!"),
        )
        .child(
            button(cx)
                .on(ev::click, move |_| {
                    set_value.update(|value| *value += step)
                })
                .child("+1"),
        )
}
