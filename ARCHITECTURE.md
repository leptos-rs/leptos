# Architecture

The goal of this document is to make it easier for contributors (and anyone
who’s interested!) to understand the architecture of the framework.

The whole Leptos framework is built from a series of layers. Each of these layers
depends on the one below it, but each can be used independently from the ones
built on top of it. While running a command like `cargo leptos new --git 
leptos-rs/start` pulls in the whole framework, it’s important to remember that
none of this is magic: each layer of that onion can be stripped away and
reimplemented, configured, or adapted as needed, incrementally.

> Everything that follows will assume you have a good working understanding
> of the framework. There will be explanations of how some parts of it work
> or fit together, but these are not docs. They assume you know what I’m
> talking about.

## The Reactive System: `leptos_reactive`

The reactive system allows you to define dynamic values (signals),
the relationships between them (derived signals and memos), and the side effects
that run in response to them (effects).

These concepts are completely independent of the DOM and can be used to drive
any kind of reactive updates. The reactive system is based on the assumption
that data is relatively cheap, and side effects are relatively expensive. Its
goal is to minimize those side effects (like updating the DOM or making a network
requests) as infrequently as possible.

The reactive system is implemented as a single data structure that exists at
runtime. In exchange for giving ownership over a value to the reactive system
(by creating a signal), you receive a `Copy + 'static` identifier for its
location in the reactive system. This enables most of the ergonomics of storing
and sharing state, the use of callback closures without lifetime issues, etc.
This is implemented by storing signals in a slotmap arena. The signal, memo,
and scope types that are exposed to users simply carry around an index into that
slotmap.

> Items owned by the reactive system are dropped when the corresponding reactive
> scope is dropped, i.e., when the component or section of the UI they’re
> created in is removed. In a sense, Leptos implements a “garbage collector”
> in which the lifetime of data is tied to the lifetime of the UI, not Rust’s
> lexical scopes.

## The DOM Renderer: `leptos_dom`

The reactive system can be used to drive any kinds of side effects. One very
common side effect is calling an imperative method, for example to update the
DOM.

The entire DOM renderer is built on top of the reactive system. It provides
a builder pattern that can be used to create DOM elements dynamically.

The renderer assumes, as a convention, that dynamic attributes, classes,
styles, and children are defined by being passed a `Fn() -> T`, where their
static equivalents just receive `T`. There’s nothing about this that is
divinely ordained, but it’s a useful convention because it allows us to use
zero-overhead derived signals as one of several ways to indicate dynamic
content.

`leptos_dom` also contains code for server-side rendering of the same
UI views to HTML, either for out-of-order streaming (`src/ssr.rs`) or
in-order streaming/async rendering (`src/ssr_in_order.rs`).

## The Macros: `leptos_macro`

It’s entirely possible to write Leptos code with no macros at all. The
`view` and `component` macros, the most common, can be replaced by
the builder syntax and simple functions (see the `counter_without_macros`
example). But the macros enable a JSX-like syntax for describing views.

This package also contains the `Params` derive macro used for typed
queries and route params in the router.

### Macro-based Optimizations

Leptos 0.0.x was built much more heavily on macros. Taking its cues  
from SolidJS, the `view` macro emitted different code for CSR, SSR, and
hydration, optimizing each. The CSR/hydrate versions worked by compiling
the view to an HTML template string, cloning that `<template>`, and
traversing the DOM to set up reactivity. The SSR version worked similarly
by compiling the static parts of the view to strings at compile time,
reducing the amount of work that needed to be done on each request.

Proc macros are hard, and this system was brittle. 0.1 introduced a
more robust renderer, including the builder syntax, and rebuilt the `view`
macro to use that builder syntax instead. It moved the optimized-but-buggy
CSR version of the macro to a more-limited `template` macro.

The `view` macro now separately optimizes SSR to use the same static-string
optimizations, which (by our benchmarks) makes Leptos about 3-4x faster
than similar Rust frontend frameworks in its HTML rendering.

> The optimization is pretty straightforward. Consider the following view:
>
> ```rust
> view! { cx,
>   <main class="text-center">
>     <div class="flex-col">
>       <button>"Click me."</button>
>       <p class="italic">"Text."</p>
>     </div>
>   </main>
> }
> ```
>
> Internally, with the builder this is something like
>
> ```rust
> Element {
>   tag: "main",
>   attrs: vec![("class", "text-center")],
>   children: vec![
> 	  Element {
> 		tag: "div",
> 		attrs: vec![("class", "flex-col")],
>       children: vec![
>         Element {
> 	        tag: "button",
> 			attrs: vec![],
> 			children: vec!["Click me"]
>         },
>         Element {
> 	        tag: "p",
> 			attrs: vec![("class", "italic")],
> 			children: vec!["Text"]
>         }
>       ]
> 	  }
>   ]
> }
> ```
>
> This is a _bunch_ of small allocations and separate strings,
> and in early 0.1 versions we used a `SmallVec` for children and
> attributes and actually caused some stack overflows.
>
> But if you look at the view itself you can see that none of this
> will _ever_ change. So we can actually optimize it at compile
> time to a single `&'static str`:
>
> ```rust
> r#"<main class="text-center">
>     <div class="flex-col">
>       <button>"Click me."</button>
>       <p class="italic">"Text."</p>
>     </div>
>   </main>"#
> ```

## Server Functions (`leptos_server`, `server_fn`, and `server_fn_macro`)

Server functions are a framework-agnostic shorthand for converting
a function, whose body can only be run on the server, into an ad hoc
REST API endpoint, and then generating code on the client to call that
endpoint when you call the function.

These are inspired by Solid/Bling’s `server$` functions, and there’s
similar work being done in a number of other JavaScript frameworks.

RPC is not a new idea, but these kinds of server functions may be.
Specifically, by using web standards (defaulting to `POST`/`GET` requests
with URL-encoded form data) they allow easy graceful degradation and the
use of the `<form>` element.

This function is split across three packages so that `server_fn` and
`server_fn_macro` can be used by other frameworks. `leptos_server`
includes some Leptos-specific reactive functionality (like actions).

## `leptos`

This package is built on and reexports most of the layers already
mentioned, and implements a number of control-flow components (`<Show/>`,
`<ErrorBoundary/>`, `<For/>`, `<Suspense/>`, `<Transition/>`) that use
public APIs of the other packages.

This is the main entrypoint for users, but is relatively light itself.

## `leptos_meta`

This package exists to allow you to work with tags normally found in
the `<head>`, from within your components.

It is implemented as a distinct package, rather than part of
`leptos_dom`, on the principle that “what can be implemented in userland,
should be.” The framework can be used without it, so it’s not in core.

## `leptos_router`

The router originates as a direct port of `solid-router`, which is the
origin of most of its terminology, architecture, and route-matching logic.

Subsequent developments (like animated routing, and managing route transitions
given the lack of `useTransition` in Leptos) have caused it to diverge
slightly from Solid’s exact code, but it is still very closely related.

The core principle here is “nested routing,” dividing a single page
into independently-rendered parts. This is described in some detail in the docs.

Like `leptos_meta`, it is implemented as a distinct package, because it
can be replaced with another router or with none. The framework can be used
without it, so it’s not in core.

## Server Integrations

The server integrations are the most “frameworky” layer of the whole framework.
These **do** assume the use of `leptos`, `leptos_router`, and `leptos_meta`.
They specifically draw routing data from `leptos_router`, and inject the
metadata from `leptos_meta` into the `<head>` appropriately.

But of course, if you one day create `leptos-helmet` and `leptos-better-router`,
you can create new server integrations that plug them into the SSR rendering
methods from `leptos_dom` instead. Everything involved is quite modular.

These packages essentially provide helpers that save the templates and user apps
from including a huge amount of boilerplate to connect the various other packages
correctly. Again, early versions of the framework examples are illustrative here
for reference: they include large amounts of manual SSR route handling, etc.

## `cargo-leptos` helpers

`leptos_config` and `leptos_hot_reload` exist to support two different features
of `cargo-leptos`, namely its configuration and its view-patching/hot-reloading 
features.

It’s important to say that the main feature `cargo-leptos` remains its ability
to conveniently tie together different build tooling, compiling your app to
WASM for the browser, building the server version, pulling in SASS and
Tailwind, etc. It is an extremely good build tool, not a magic formula. Each
of the examples includes instructions for how to run the examples without
`cargo-leptos`.
