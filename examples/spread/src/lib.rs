use leptos::{
    attr::id,
    ev::{self, on},
    prelude::*,
    // TODO clean up import here
    tachys::html::class::class,
};

/// Demonstrates how attributes and event handlers can be spread onto elements.
#[component]
pub fn SpreadingExample() -> impl IntoView {
    fn alert(msg: impl AsRef<str>) {
        let _ = window().alert_with_message(msg.as_ref());
    }

    // TODO support data- attributes better
    let attrs_only = class("foo");
    let event_handlers_only = on(ev::click, move |_e: ev::MouseEvent| {
        alert("event_handlers_only clicked");
    });
    let combined = (
        class("bar"),
        on(ev::click, move |_e: ev::MouseEvent| {
            alert("combined clicked");
        }),
    );
    let partial_attrs = (id("snood"), class("baz"));
    let partial_event_handlers = on(ev::click, move |_e: ev::MouseEvent| {
        alert("partial_event_handlers clicked");
    });

    view! {
        <p>
            "You can spread any valid attribute, including a tuple of attributes, with the {..attr} syntax"
        </p>
        <div {..attrs_only.clone()}>"<div {..attrs_only} />"</div>

        <div {..event_handlers_only.clone()}>"<div {..event_handlers_only} />"</div>

        <div {..combined.clone()}>"<div {..combined} />"</div>

        <div {..partial_attrs.clone()} {..partial_event_handlers.clone()}>
            "<div {..partial_attrs} {..partial_event_handlers} />"
        </div>

        <hr/>

        <p>
            "The .. is not required to spread; you can pass any valid attribute in a block by itself."
        </p>
        <div {attrs_only}>"<div {attrs_only} />"</div>

        <div {event_handlers_only}>"<div {event_handlers_only} />"</div>

        <div {combined}>"<div {combined} />"</div>

        <div {partial_attrs} {partial_event_handlers}>
            "<div {partial_attrs} {partial_event_handlers} />"
        </div>
    }
    // TODO check below
    // Overwriting an event handler, here on:click, will result in a panic in debug builds. In release builds, the initial handler is kept.
    // If spreading is used, prefer manually merging event handlers in the binding list instead.
    //<div {..mixed} on:click=|_e| { alert("I will never be seen..."); }>
    //    "with overwritten click handler"
    //</div>
}
