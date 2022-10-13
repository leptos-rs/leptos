**Please note:** This framework is in active development. I'm keeping it in a cycle of 0.0.x releases at the moment to indicate that it’s not even ready for its 0.1.0. Active work is being done on documentation and features, and APIs should not necessarily be considered stable. At the same time, it is more than a toy project or proof of concept, and I am actively using it for my own application development.

<img src="https://raw.githubusercontent.com/gbj/leptos/main/docs/logos/logo.svg" alt="Leptos Logo" style="width: 100%; height: auto; display: block; margin: auto;">

[![crates.io](https://img.shields.io/crates/v/leptos.svg)](https://crates.io/crates/leptos)
[![docs.rs](https://docs.rs/leptos/badge.svg)](https://docs.rs/leptos)

# Leptos

```rust
use leptos::*;

#[component]
pub fn SimpleCounter(cx: Scope, initial_value: i32) -> Element {
    // create a reactive signal with the initial value
    let (value, set_value) = create_signal(cx, inital_value);

    // create event handlers for our buttons
    // note that `value` and `set_value` are `Copy`, so it's super easy to move them into closures
    let clear = move |_| set_value(0);
    let decrement = move |_| set_value.update(|value| *value -= 1);
    let increment = move |_| set_value.update(|value| *value += 1);

    // this JSX is compiled to an HTML template string for performance
    view! {
        cx,
        <div>
            <button on:click=clear>"Clear"</button>
            <button on:click=decrement>"-1"</button>
            <span>"Value: " {move || value().to_string()} "!"</span>
            <button on:click=increment>"+1"</button>
        </div>
    }
}

// Easy to use with Trunk (trunkrs.dev) or with a simple wasm-bindgen setup
pub fn main() {
    mount_to_body(|cx| view! { cx,  <SimpleCounter initial_value=3> })
}

```

## About the Framework

Leptos is a full-stack, isomorphic Rust web framework leveraging fine-grained reactivity to build declarative user interfaces.

## What does that mean?

- **Full-stack**: Leptos can be used to build apps that run in the browser (_client-side rendering_), on the server (_server-side rendering_), or by rendering HTML on the server and then adding interactivity in the browser (_hydration_). This includes support for *HTTP streaming* of both data (`Resource`s) and HTML (out-of-order streaming of `<Suspense/>` components.)
- **Isomorphic**: The same application code and business logic are compiled to run on the client and server, with seamless integration. You can write your server-only logic (database requests, authentication etc.) alongside the client-side components that will consume it, and let Leptos manage the data loading without the need to manually create APIs to consume.
- **Web**: Leptos is built on the Web platform and Web standards. Whenever possible, we use Web essentials (like links and forms) and build on top of them rather than trying to replace them.
- **Framework**: Leptos provides most of what you need to build a modern web app: a reactive system, templating library, and a router that works on both the server and client side.
- **Fine-grained reactivity**: The entire framework is build from reactive primitives. This allows for extremely performant code with minimal overhead: when a reactive signal’s value changes, it can update a single text node, toggle a single class, or remove an element from the DOM without any other code running. (_So, no virtual DOM!_)
- **Declarative**: Tell Leptos how you want the page to look, and let the framework tell the browser how to do it.

## Learn more

Here are some resources for learning more about Leptos:

- [Examples](https://github.com/gbj/leptos/tree/main/examples)
- [API Documentation](https://docs.rs/leptos/latest/leptos/) (in progress)
- Leptos Guide (in progress)
