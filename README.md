<picture>
    <source srcset="https://raw.githubusercontent.com/leptos-rs/leptos/main/docs/logos/Leptos_logo_pref_dark_RGB.svg" media="(prefers-color-scheme: dark)">
    <img src="https://raw.githubusercontent.com/leptos-rs/leptos/main/docs/logos/Leptos_logo_RGB.svg" alt="Leptos Logo">
</picture>

[![crates.io](https://img.shields.io/crates/v/leptos.svg)](https://crates.io/crates/leptos)
[![docs.rs](https://docs.rs/leptos/badge.svg)](https://docs.rs/leptos)
[![Discord](https://img.shields.io/discord/1031524867910148188?color=%237289DA&label=discord)](https://discord.gg/YdRAhS7eQB)
[![Matrix](https://img.shields.io/badge/Matrix-leptos-grey?logo=matrix&labelColor=white&logoColor=black)](https://matrix.to/#/#leptos:matrix.org)

[Website](https://leptos.dev) | [Book](https://leptos-rs.github.io/leptos/) | [Docs.rs](https://docs.rs/leptos/latest/leptos/) | [Playground](https://codesandbox.io/p/sandbox/leptos-rtfggt?file=%2Fsrc%2Fmain.rs%3A1%2C1) | [Discord](https://discord.gg/YdRAhS7eQB)

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
    view! { cx,
        <div>
            <button on:click=clear>"Clear"</button>
            <button on:click=decrement>"-1"</button>
            <span>"Value: " {value} "!"</span>
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

> Note: The `nightly` feature is present on the main branch version right now, but not in 0.3.x. For 0.3.x, nightly is the default and `stable` has a special feature.

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

The APIs are basically settled. We’re adding new features, but we’re very happy with where the type system and patterns have landed. I would not expect major breaking changes to your code to adapt to future releases. The sorts of breaking changes that we discuss are things like “Oh yeah, that function should probably take `cx` as its argument...” not major changes to the way you write your application.

2. **Are there bugs?**

Yes, I’m sure there are. You can see from the state of our issue tracker over time that there aren’t that _many_ bugs and they’re usually resolved pretty quickly. But for sure, there may be moments where you encounter something that requires a fix at the framework level, which may not be immediately resolved.

3. **Am I a consumer or a contributor?**

This may be the big one: “production ready” implies a certain orientation to a library: that you can basically use it, without any special knowledge of its internals or ability to contribute. Everyone has this at some level in their stack: for example I (@gbj) don’t have the capacity or knowledge to contribute to something like `wasm-bindgen` at this point: I simply rely on it to work.

There are several people in the community using Leptos right now for internal apps at work, who have also become significant contributors. I think this is the right level of production use for now. There may be missing features that you need, and you may end up building them! But for internal apps, if you’re willing to build and contribute missing pieces along the way, the framework is definitely usable right now.

### Can I use this for native GUI?

Sure! Obviously the `view` macro is for generating DOM nodes but you can use the reactive system to drive native any GUI toolkit that uses the same kind of object-oriented, event-callback-based framework as the DOM pretty easily. The principles are the same:

- Use signals, derived signals, and memos to create your reactive system
- Create GUI widgets
- Use event listeners to update signals
- Create effects to update the UI

I've put together a [very simple GTK example](https://github.com/leptos-rs/leptos/blob/main/examples/gtk/src/main.rs) so you can see what I mean.

### How is this different from Yew/Dioxus?

On the surface level, these libraries may seem similar. Yew is, of course, the most mature Rust library for web UI development and has a huge ecosystem. Dioxus is similar in many ways, being heavily inspired by React. Here are some conceptual differences between Leptos and these frameworks:

- **VDOM vs. fine-grained:** Yew is built on the virtual DOM (VDOM) model: state changes cause components to re-render, generating a new virtual DOM tree. Yew diffs this against the previous VDOM, and applies those patches to the actual DOM. Component functions rerun whenever state changes. Leptos takes an entirely different approach. Components run once, creating (and returning) actual DOM nodes and setting up a reactive system to update those DOM nodes.
- **Performance:** This has huge performance implications: Leptos is simply much faster at both creating and updating the UI than Yew is. (Dioxus has made huge advances in performance with its recent 0.3 release, and is now roughly on par with Leptos.)
- **Mental model:** Adopting fine-grained reactivity also tends to simplify the mental model. There are no surprising component re-renders because there are no re-renders. You can call functions, create timeouts, etc. within the body of your component functions because they won’t be re-run. You don’t need to think about manual dependency tracking for effects; fine-grained reactivity tracks dependencies automatically.

### How is this different from Sycamore?

Conceptually, these two frameworks are very similar: because both are built on fine-grained reactivity, most apps will end up looking very similar between the two, and Sycamore or Leptos apps will both look a lot like SolidJS apps, in the same way that Yew or Dioxus can look a lot like React.

There are some practical differences that make a significant difference:

- **Templating:** Leptos uses a JSX-like template format (built on [syn-rsx](https://github.com/stoically/syn-rsx)) for its `view` macro. Sycamore offers the choice of its own templating DSL or a builder syntax.
- **Server integration:** Leptos provides primitives that encourage HTML streaming and allow for easy async integration and RPC calls, even without WASM enabled, making it easy to opt into integrations between your frontend and backend code without pushing you toward any particular metaframework patterns.
- **Read-write segregation:** Leptos, like Solid, encourages read-write segregation between signal getters and setters, so you end up accessing signals with tuples like `let (count, set_count) = create_signal(cx, 0);` _(If you prefer or if it's more convenient for your API, you can use [`create_rw_signal`](https://docs.rs/leptos/latest/leptos/fn.create_rw_signal.html) to give a unified read/write signal.)_
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
