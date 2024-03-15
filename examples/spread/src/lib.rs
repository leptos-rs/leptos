use leptos::*;

/// Demonstrates how attributes and event handlers can be spread onto elements.
#[component]
pub fn SpreadingExample() -> impl IntoView {
    fn alert(msg: impl AsRef<str>) {
        let _ = window().alert_with_message(msg.as_ref());
    }

    let attrs_only: Vec<(&'static str, Attribute)> = vec![
        ("data-foo", Attribute::String(Oco::Borrowed("42"))),
        ("aria-disabled", Attribute::String(Oco::Borrowed("false"))),
    ];

    let event_handlers_only: Vec<EventHandlerFn> = vec![
        EventHandlerFn::Click(Box::new(|_e: ev::MouseEvent| {
            alert("event_handlers_only clicked");
        })),
    ];

    let mixed: Vec<Prop> = vec![
        ("data-foo", Attribute::String(Oco::Borrowed("123"))).into(),
        ("aria-disabled", Attribute::String(Oco::Borrowed("true"))).into(),
        EventHandlerFn::Click(Box::new(|_e: ev::MouseEvent| {
            alert("mixed clicked");
        })).into(),
    ];

    view! {
        <div {..attrs_only}>
            "attrs_only"
        </div>

        <div {..event_handlers_only}>
            "event_handlers_only"
        </div>

        <div {..mixed}>
            "mixed"
        </div>

        // Overwriting an event handler, here on:click, will result in a panic in debug builds. In release builds, the initial handler is kept.
        // If spreading is used, prefer manually merging event handlers in the prop list instead.
        //<div {..mixed} on:click=|_e| { alert("I will never be seen..."); }>
        //    "with overwritten click handler"
        //</div>
    }
}
