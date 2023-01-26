use leptos::{ev, *};

pub struct Props {
    /// The starting value for the counter
    pub initial_value: i32,
    /// The change that should be applied each time the button is clicked.
    pub step: i32,
}

/// A simple counter view.
pub fn view(cx: Scope, props: Props) -> impl IntoView {
    let Props {
        initial_value,
        step,
    } = props;
    let (value, set_value) = create_signal(cx, initial_value);

    div(cx)
        .child((
            cx,
            button(cx)
                .on(ev::click, move |_| set_value.update(|value| *value = 0))
                .child((cx, "Clear")),
        ))
        .child((
            cx,
            button(cx)
                .on(ev::click, move |_| set_value.update(|value| *value -= step))
                .child((cx, "-1")),
        ))
        .child((
            cx,
            span(cx)
                .child((cx, "Value: "))
                .child((cx, move || value.get()))
                .child((cx, "!")),
        ))
        .child((
            cx,
            button(cx)
                .on(ev::click, move |_| set_value.update(|value| *value += step))
                .child((cx, "+1")),
        ))
}
