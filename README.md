**NOTE: We're in the middle of merging changes and making fixes to support our upcoming `0.1.0` release. Some of the examples may be in a broken state. You can continue using the `0.0` releases with no issues.**

<img src="https://raw.githubusercontent.com/gbj/leptos/main/docs/logos/logo.svg" alt="Leptos Logo" style="width: 100%; height: auto; display: block; margin: auto;">

[![crates.io](https://img.shields.io/crates/v/leptos.svg)](https://crates.io/crates/leptos)
[![docs.rs](https://docs.rs/leptos/badge.svg)](https://docs.rs/leptos)
[![Discord](https://img.shields.io/discord/1031524867910148188?color=%237289DA&label=discord)](https://discord.gg/YdRAhS7eQB)

# Leptos

```rust
use leptos::*;

#[component]
pub fn SimpleCounter(cx: Scope, initial_value: i32) -> impl IntoView {
    // create a reactive signal with the initial value
    let (value, set_value) = create_signal(cx, initial_value);

    // create event handlers for our buttons
    // note that `value` and `set_value` are `Copy`, so it's super easy to move them into closures
    let clear = move |_| set_value(0);
    let decrement = move |_| set_value.update(|value| *value -= 1);
    let increment = move |_| set_value.update(|value| *value += 1);

    // create user interfaces with the declarative `view!` macro
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
    mount_to_body(|cx| view! { cx,  <SimpleCounter initial_value=3 /> })
}

```

## About the Framework

Leptos is a full-stack, isomorphic Rust web framework leveraging fine-grained reactivity to build declarative user interfaces.

## What does that mean?

- **Full-stack**: Leptos can be used to build apps that run in the browser (_client-side rendering_), on the server (_server-side rendering_), or by rendering HTML on the server and then adding interactivity in the browser (_hydration_). This includes support for _HTTP streaming_ of both data (`Resource`s) and HTML (out-of-order streaming of `<Suspense/>` components.)
- **Isomorphic**: Leptos provides primitives to write isomorphic server functions, i.e., functions that can be called with the “same shape” on the client or server, but only run on the server. This means you can write your server-only logic (database requests, authentication etc.) alongside the client-side components that will consume it, and call server functions as if they were running in the browser.
- **Web**: Leptos is built on the Web platform and Web standards. The router is designed to use Web fundamentals (like links and forms) and build on top of them rather than trying to replace them.
- **Framework**: Leptos provides most of what you need to build a modern web app: a reactive system, templating library, and a router that works on both the server and client side.
- **Fine-grained reactivity**: The entire framework is built from reactive primitives. This allows for extremely performant code with minimal overhead: when a reactive signal’s value changes, it can update a single text node, toggle a single class, or remove an element from the DOM without any other code running. (_So, no virtual DOM!_)
- **Declarative**: Tell Leptos how you want the page to look, and let the framework tell the browser how to do it.

## Learn more

Here are some resources for learning more about Leptos:

- [Examples](https://github.com/gbj/leptos/tree/main/examples)
- [API Documentation](https://docs.rs/leptos/latest/leptos/)
- [Common Bugs](https://github.com/gbj/leptos/tree/main/docs/COMMON_BUGS.md) (and how to fix them!)
- Leptos Guide (in progress)


## `nightly` Note

Most of the examples assume you’re using `nightly` Rust.

To set up your Rust toolchain using `nightly` (and add the ability to compile Rust to WebAssembly, if you haven’t already)

```
rustup toolchain install nightly
rustup default nightly
rustup target add wasm32-unknown-unknown
```

If you’re on `stable`, note the following:

1. You need to enable the `"stable"` flag in `Cargo.toml`: `leptos = { version = "0.1.0-alpha", features = ["stable"] }`
2. `nightly` enables the function call syntax for accessing and setting signals. If you’re using `stable`,
   you’ll just call `.get()`, `.set()`, or `.update()` manually. Check out the
   [`counters-stable` example](https://github.com/gbj/leptos/blob/main/examples/counters-stable/src/main.rs)
   for examples of the correct API.

## `cargo-leptos`

[`cargo-leptos`](https://github.com/akesson/cargo-leptos) is a build tool that's designed to make it easy to build apps that run on both the client and the server, with seamless integration. The best way to get started with a real Leptos project right now is to use `cargo-leptos` and our [starter template](https://github.com/leptos-rs/start).

```bash
cargo install cargo-leptos
cargo leptos new --git https://github.com/leptos-rs/start
cd [your project name]
cargo leptos watch
```

## FAQs

### Can I use this for native GUI?

Sure! Obviously the `view` macro is for generating DOM nodes but you can use the reactive system to drive native any GUI toolkit that uses the same kind of object-oriented, event-callback-based framework as the DOM pretty easily. The principles are the same:

- Use signals, derived signals, and memos to create your reactive system
- Create GUI widgets
- Use event listeners to update signals
- Create effects to update the UI

I've put together a [very simple GTK example](https://github.com/gbj/leptos/blob/main/examples/gtk/src/main.rs) so you can see what I mean.

### How is this different from Yew/Dioxus?

On the surface level, these libraries may seem similar. Yew is, of course, the most mature Rust library for web UI development and has a huge ecosystem. Dioxus is similar in many ways, being heavily inspired by React. Here are some conceptual differences between Leptos and these frameworks:

- **VDOM vs. fine-grained:** Yew is built on the virtual DOM (VDOM) model: state changes cause components to re-render, generating a new virtual DOM tree. Yew diffs this against the previous VDOM, and applies those patches to the actual DOM. Component functions rerun whenever state changes. Leptos takes an entirely different approach. Components run once, creating (and returning) actual DOM nodes and setting up a reactive system to update those DOM nodes.
- **Performance:** This has huge performance implications: Leptos is simply _much_ faster at both creating and updating the UI than Yew is.
- **Mental model:** Adopting fine-grained reactivity also tends to simplify the mental model. There are no surprising component re-renders because there are no re-renders. Your app can be divided into components based on what makes sense for your app, because they have no performance implications.

### How is this different from Sycamore?

Conceptually, these two frameworks are very similar: because both are built on fine-grained reactivity, most apps will end up looking very similar between the two, and Sycamore or Leptos apps will both look a lot like SolidJS apps, in the same way that Yew or Dioxus can look a lot like React.

There are some practical differences that make a significant difference:

- **Maturity:** Sycamore is obviously a much more mature and stable library with a larger ecosystem.
- **Templating:** Leptos uses a JSX-like template format (built on [syn-rsx](https://github.com/stoically/syn-rsx)) for its `view` macro. Sycamore offers the choice of its own templating DSL or a builder syntax.
- **Template node cloning:** Leptos's `view` macro compiles to a static HTML string and a set of instructions of how to assign its reactive values. This means that at runtime, Leptos can clone a `<template>` node rather than calling `document.createElement()` to create DOM nodes. This is a _significantly_ faster way of rendering components.
- **Read-write segregation:** Leptos, like Solid, encourages read-write segregation between signal getters and setters, so you end up accessing signals with tuples like `let (count, set_count) = create_signal(cx, 0);` _(If you prefer or if it's more convenient for your API, you can use `create_rw_signal` to give a unified read/write signal.)_
- **Signals are functions:** In Leptos, you can call a signal to access it rather than calling a specific method (so, `count()` instead of `count.get()`) This creates a more consistent mental model: accessing a reactive value is always a matter of calling a function. For example:

  ```rust
  let (count, set_count) = create_signal(cx, 0); // a signal
  let double_count = move || count() * 2; // a derived signal
  let memoized_count = create_memo(cx, move |_| count() * 3); // a memo
  // all are accessed by calling them
  assert_eq!(count(), 0);
  assert_eq!(double_count(), 0);
  assert_eq!(memoized_count(), 0);
  // this function can accept any of those signals
  fn do_work_on_signal(my_signal: impl Fn() -> i32) { ... }
  ```

- **Signals and scopes are `'static`:** Both Leptos and Sycamore ease the pain of moving signals in closures (in particular, event listeners) by making them `Copy`, to avoid the `{ let count = count.clone(); move |_| ... }` that's very familiar in Rust UI code. Sycamore does this by using bump allocation to tie the lifetimes of its signals to its scopes: since references are `Copy`, `&'a Signal<T>` can be moved into a closure. Leptos does this by using arena allocation and passing around indices: types like `ReadSignal<T>`, `WriteSignal<T>`, and `Memo<T>` are actually wrappers for indices into an arena. This means that both scopes and signals are both `Copy` and `'static` in Leptos, which means that they can be moved easily into closures without adding lifetime complexity.
