use leptos::*;

/// Demonstrates how attributes and event handlers can be spread onto elements.
#[component]
pub fn SpreadingExample() -> impl IntoView {
    fn alert(msg: impl AsRef<str>) {
        let _ = window().alert_with_message(msg.as_ref());
    }

    let attrs_only: Vec<(&'static str, Attribute)> =
        vec![("data-foo", "42".into_attribute())];

    let event_handlers_only: Vec<EventHandlerFn> =
        vec![EventHandlerFn::Click(Box::new(|_e: ev::MouseEvent| {
            alert("event_handlers_only clicked");
        }))];

    let combined: Vec<Binding> = vec![
        ("data-foo", "123".into_attribute()).into(),
        EventHandlerFn::Click(Box::new(|_e: ev::MouseEvent| {
            alert("combined clicked");
        }))
        .into(),
    ];

    let partial_attrs: Vec<(&'static str, Attribute)> =
        vec![("data-foo", "11".into_attribute())];

    let partial_event_handlers: Vec<EventHandlerFn> =
        vec![EventHandlerFn::Click(Box::new(|_e: ev::MouseEvent| {
            alert("partial_event_handlers clicked");
        }))];

    view! {
        <div {..attrs_only}>
            "<div {..attrs_only} />"
        </div>

        <div {..event_handlers_only}>
            "<div {..event_handlers_only} />"
        </div>

        <div {..combined}>
            "<div {..combined} />"
        </div>

        <div {..partial_attrs} {..partial_event_handlers}>
            "<div {..partial_attrs} {..partial_event_handlers} />"
        </div>

        // Overwriting an event handler, here on:click, will result in a panic in debug builds. In release builds, the initial handler is kept.
        // If spreading is used, prefer manually merging event handlers in the binding list instead.
        //<div {..mixed} on:click=|_e| { alert("I will never be seen..."); }>
        //    "with overwritten click handler"
        //</div>
    }
}
