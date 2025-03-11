<picture>
    <source srcset="https://raw.githubusercontent.com/leptos-rs/leptos/main/docs/logos/Leptos_logo_pref_dark_RGB.svg" media="(prefers-color-scheme: dark)">
    <img src="https://raw.githubusercontent.com/leptos-rs/leptos/main/docs/logos/Leptos_logo_RGB.svg" alt="Leptos Logo">
</picture>

[![crates.io](https://img.shields.io/crates/v/leptos.svg)](https://crates.io/crates/leptos)
[![docs.rs](https://docs.rs/leptos/badge.svg)](https://docs.rs/leptos)
![Crates.io MSRV](https://img.shields.io/crates/msrv/leptos)
[![Discord](https://img.shields.io/discord/1031524867910148188?color=%237289DA&label=discord)](https://discord.gg/YdRAhS7eQB)
[![Matrix](https://img.shields.io/badge/Matrix-leptos-grey?logo=matrix&labelColor=white&logoColor=black)](https://matrix.to/#/#leptos:matrix.org)

[Website](https://leptos.dev) | [Book](https://leptos-rs.github.io/leptos/) | [Docs.rs](https://docs.rs/leptos/latest/leptos/) | [Playground](https://codesandbox.io/p/sandbox/leptos-rtfggt?file=%2Fsrc%2Fmain.rs%3A1%2C1) | [Discord](https://discord.gg/YdRAhS7eQB)

You can find a list of useful libraries and example projects at [`awesome-leptos`](https://github.com/leptos-rs/awesome-leptos).

# Leptos

```rust
use leptos::*;

#[component]
pub fn SimpleCounter(initial_value: i32) -> impl IntoView {
    // create a reactive signal with the initial value
    let (value, set_value) = signal(initial_value);

    // create event handlers for our buttons
    // note that `value` and `set_value` are `Copy`, so it's super easy to move them into closures
    let clear = move |_| set_value(0);
    let decrement = move |_| set_value.update(|value| *value -= 1);
    let increment = move |_| set_value.update(|value| *value += 1);

    // create user interfaces with the declarative `view!` macro
    view! {
        <div>
            <button on:click=clear>Clear</button>
            <button on:click=decrement>-1</button>
            // text nodes can be quoted or unquoted
            <span>"Value: " {value} "!"</span>
            <button on:click=increment>+1</button>
        </div>
    }
}

// we also support a builder syntax rather than the JSX-like `view` macro
#[component]
pub fn SimpleCounterWithBuilder(initial_value: i32) -> impl IntoView {
    use leptos::html::*;

    let (value, set_value) = signal(initial_value);
    let clear = move |_| set_value(0);
    let decrement = move |_| set_value.update(|value| *value -= 1);
    let increment = move |_| set_value.update(|value| *value += 1);

    // the `view` macro above expands to this builder syntax
    div().child((
        button().on(ev::click, clear).child("Clear"),
        button().on(ev::click, decrement).child("-1"),
        span().child(("Value: ", value, "!")),
        button().on(ev::click, increment).child("+1")
    ))
}

// Easy to use with Trunk (trunkrs.dev) or with a simple wasm-bindgen setup
pub fn main() {
    mount_to_body(|| view! {
        <SimpleCounter initial_value=3 />
    })
}
```

## About the Framework

Leptos is a full-stack, isomorphic Rust web framework leveraging fine-grained reactivity to build declarative user interfaces.

## What does that mean?

- **Full-stack**: Leptos can be used to build apps that run in the browser (client-side rendering), on the server (server-side rendering), or by rendering HTML on the server and then adding interactivity in the browser (server-side rendering with hydration). This includes support for HTTP streaming of both data ([`Resource`s](https://docs.rs/leptos/latest/leptos/struct.Resource.html)) and HTML (out-of-order or in-order streaming of [`<Suspense/>`](https://docs.rs/leptos/latest/leptos/fn.Suspense.html) components.)
- **Isomorphic**: Leptos provides primitives to write isomorphic [server functions](https://docs.rs/leptos_server/0.2.5/leptos_server/index.html), i.e., functions that can be called with the “same shape” on the client or server, but only run on the server. This means you can write your server-only logic (database requests, authentication etc.) alongside the client-side components that will consume it, and call server functions as if they were running in the browser, without needing to create and maintain a separate REST or other API.
- **Web**: Leptos is built on the Web platform and Web standards. The [router](https://docs.rs/leptos_router/latest/leptos_router/) is designed to use Web fundamentals (like links and forms) and build on top of them rather than trying to replace them.
- **Framework**: Leptos provides most of what you need to build a modern web app: a reactive system, templating library, and a router that works on both the server and client side.
- **Fine-grained reactivity**: The entire framework is built from reactive primitives. This allows for extremely performant code with minimal overhead: when a reactive signal’s value changes, it can update a single text node, toggle a single class, or remove an element from the DOM without any other code running. (So, no virtual DOM overhead!)
- **Declarative**: Tell Leptos how you want the page to look, and let the framework tell the browser how to do it.

## Learn more

Here are some resources for learning more about Leptos:

- [Book](https://leptos-rs.github.io/leptos/) (work in progress)
- [Examples](https://github.com/leptos-rs/leptos/tree/main/examples)
- [API Documentation](https://docs.rs/leptos/latest/leptos/)
- [Common Bugs](https://github.com/leptos-rs/leptos/tree/main/docs/COMMON_BUGS.md) (and how to fix them!)

## `nightly` Note

Most of the examples assume you’re using `nightly` version of Rust and the `nightly` feature of Leptos. To use `nightly` Rust, you can either set your toolchain globally or on per-project basis.

To set `nightly` as a default toolchain for all projects (and add the ability to compile Rust to WebAssembly, if you haven’t already):

```
rustup toolchain install nightly
rustup default nightly
rustup target add wasm32-unknown-unknown
```

If you'd like to use `nightly` only in your Leptos project however, add [`rust-toolchain.toml`](https://rust-lang.github.io/rustup/overrides.html#the-toolchain-file) file with the following content:

```toml
[toolchain]
channel = "nightly"
targets = ["wasm32-unknown-unknown"]
```

The `nightly` feature enables the function call syntax for accessing and setting signals, as opposed to `.get()` and `.set()`. This leads to a consistent mental model in which accessing a reactive value of any kind (a signal, memo, or derived signal) is always represented as a function call. This is only possible with nightly Rust and the `nightly` feature.

## `cargo-leptos`

[`cargo-leptos`](https://github.com/leptos-rs/cargo-leptos) is a build tool that's designed to make it easy to build apps that run on both the client and the server, with seamless integration. The best way to get started with a real Leptos project right now is to use `cargo-leptos` and our starter templates for [Actix](https://github.com/leptos-rs/start) or [Axum](https://github.com/leptos-rs/start-axum).

```bash
cargo install cargo-leptos
cargo leptos new --git https://github.com/leptos-rs/start
cd [your project name]
cargo leptos watch
```

Open browser to [http://localhost:3000/](http://localhost:3000/).

## FAQs

### What’s up with the name?

_Leptos_ (λεπτός) is an ancient Greek word meaning “thin, light, refined, fine-grained.” To me, a classicist and not a dog owner, it evokes the lightweight reactive system that powers the framework. I've since learned the same word is at the root of the medical term “leptospirosis,” a blood infection that affects humans and animals... My bad. No dogs were harmed in the creation of this framework.

### Is it production ready?

People usually mean one of three things by this question.

1. **Are the APIs stable?** i.e., will I have to rewrite my whole app from Leptos 0.1 to 0.2 to 0.3 to 0.4, or can I write it now and benefit from new features and updates as new versions come?

The APIs are basically settled. We’re adding new features, but we’re very happy with where the type system and patterns have landed. I would not expect major breaking changes to your code to adapt to future releases, in terms of architecture.

2. **Are there bugs?**

Yes, I’m sure there are. You can see from the state of our issue tracker over time that there aren’t that _many_ bugs and they’re usually resolved pretty quickly. But for sure, there may be moments where you encounter something that requires a fix at the framework level, which may not be immediately resolved.

3. **Am I a consumer or a contributor?**

This may be the big one: “production ready” implies a certain orientation to a library: that you can basically use it, without any special knowledge of its internals or ability to contribute. Everyone has this at some level in their stack: for example I (@gbj) don’t have the capacity or knowledge to contribute to something like `wasm-bindgen` at this point: I simply rely on it to work.

There are several people in the community using Leptos right now for internal apps at work, who have also become significant contributors. I think this is the right level of production use for now. There may be missing features that you need, and you may end up building them! But for internal apps, if you’re willing to build and contribute missing pieces along the way, the framework is definitely usable right now.

### Can I use this for native GUI?

Sure! Obviously the `view` macro is for generating DOM nodes but you can use the reactive system to drive any native GUI toolkit that uses the same kind of object-oriented, event-callback-based framework as the DOM pretty easily. The principles are the same:

- Use signals, derived signals, and memos to create your reactive system
- Create GUI widgets
- Use event listeners to update signals
- Create effects to update the UI

The 0.7 update originally set out to create a "generic rendering" approach that would allow us to reuse most of the same view logic to do all of the above. Unfortunately, this has had to be shelved for now due to difficulties encountered by the Rust compiler when building larger-scale applications with the number of generics spread throughout the codebase that this required. It's an approach I'm looking forward to exploring again in the future; feel free to reach out if you're interested in this kind of work.

### How is this different from Yew?

Yew is the most-used library for Rust web UI development, but there are several differences between Yew and Leptos, in philosophy, approach, and performance.

- **VDOM vs. fine-grained:** Yew is built on the virtual DOM (VDOM) model: state changes cause components to re-render, generating a new virtual DOM tree. Yew diffs this against the previous VDOM, and applies those patches to the actual DOM. Component functions rerun whenever state changes. Leptos takes an entirely different approach. Components run once, creating (and returning) actual DOM nodes and setting up a reactive system to update those DOM nodes.
- **Performance:** This has huge performance implications: Leptos is simply much faster at both creating and updating the UI than Yew is.
- **Server integration:** Yew was created in an era in which browser-rendered single-page apps (SPAs) were the dominant paradigm. While Leptos supports client-side rendering, it also focuses on integrating with the server side of your application via server functions and multiple modes of serving HTML, including out-of-order streaming.

### How is this different from Dioxus?

Like Leptos, Dioxus is a framework for building UIs using web technologies. However, there are significant differences in approach and features.

- **VDOM vs. fine-grained:** While Dioxus has a performant virtual DOM (VDOM), it still uses coarse-grained/component-scoped reactivity: changing a stateful value reruns the component function and diffs the old UI against the new one. Leptos components use a different mental model, creating (and returning) actual DOM nodes and setting up a reactive system to update those DOM nodes.
- **Web vs. desktop priorities:** Dioxus uses Leptos server functions in its fullstack mode, but does not have the same `<Suspense>`-based support for things like streaming HTML rendering, or share the same focus on holistic web performance. Leptos tends to prioritize holistic web performance (streaming HTML rendering, smaller WASM binary sizes, etc.), whereas Dioxus has an unparalleled experience when building desktop apps, because your application logic runs as a native Rust binary.

### How is this different from Sycamore?

Sycamore and Leptos are both heavily influenced by SolidJS. At this point, Leptos has a larger community and ecosystem and is more actively developed. Other differences:

- **Templating DSLs:** Sycamore uses a custom templating language for its views, while Leptos uses a JSX-like template format.
- **`'static` signals:** One of Leptos’s main innovations was the creation of `Copy + 'static` signals, which have excellent ergonomics. Sycamore is in the process of adopting the same pattern, but this is not yet released.
- **Perseus vs. server functions:** The Perseus metaframework provides an opinionated way to build Sycamore apps that include server functionality. Leptos instead provides primitives like server functions in the core of the framework.
