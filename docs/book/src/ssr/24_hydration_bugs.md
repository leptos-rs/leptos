# Hydration Bugs _(and how to avoid them)_

## A Thought Experiment

Let’s try an experiment to test your intuitions. Open up an app you’re server-rendering with `cargo-leptos`. (If you’ve just been using `trunk` so far to play with examples, go [clone a `cargo-leptos` template](./21_cargo_leptos.md) just for the sake of this exercise.)

Put a log somewhere in your root component. (I usually call mine `<App/>`, but anything will do.)

```rust
#[component]
pub fn App() -> impl IntoView {
	logging::log!("where do I run?");
	// ... whatever
}
```

And let’s fire it up

```bash
cargo leptos watch
```

Where do you expect `where do I run?` to log?

- In the command line where you’re running the server?
- In the browser console when you load the page?
- Neither?
- Both?

Try it out.

...

...

...

Okay, consider the spoiler alerted.

You’ll notice of course that it logs in both places, assuming everything goes according to plan. In fact on the server it logs twice—first during the initial server startup, when Leptos renders your app once to extract the route tree, then a second time when you make a request. Each time you reload the page, `where do I run?` should log once on the server and once on the client.

If you think about the description in the last couple sections, hopefully this makes sense. Your application runs once on the server, where it builds up a tree of HTML which is sent to the client. During this initial render, `where do I run?` logs on the server.

Once the WASM binary has loaded in the browser, your application runs a second time, walking over the same user interface tree and adding interactivity.

> Does that sound like a waste? It is, in a sense. But reducing that waste is a genuinely hard problem. It’s what some JS frameworks like Qwik are intended to solve, although it’s probably too early to tell whether it’s a net performance gain as opposed to other approaches.

## The Potential for Bugs

Okay, hopefully all of that made sense. But what does it have to do with the title of this chapter, which is “Hydration bugs (and how to avoid them)”?

Remember that the application needs to run on both the server and the client. This generates a few different sets of potential issues you need to know how to avoid.

### Mismatches between server and client code

One way to create a bug is by creating a mismatch between the HTML that’s sent down by the server and what’s rendered on the client. It’s actually fairly hard to do this unintentionally, I think (at least judging by the bug reports I get from people.) But imagine I do something like this

```rust
#[component]
pub fn App() -> impl IntoView {
    let data = if cfg!(target_arch = "wasm32") {
        vec![0, 1, 2]
    } else {
        vec![]
    };
    data.into_iter()
        .map(|value| view! { <span>{value}</span> })
        .collect_view()
}
```

In other words, if this is being compiled to WASM, it has three items; otherwise it’s empty.

When I load the page in the browser, I see nothing. If I open the console I see a bunch of warnings:

```
element with id 0-3 not found, ignoring it for hydration
element with id 0-4 not found, ignoring it for hydration
element with id 0-5 not found, ignoring it for hydration
component with id _0-6c not found, ignoring it for hydration
component with id _0-6o not found, ignoring it for hydration
```

The WASM version of your app, running in the browser, expects to find three items; but the HTML has none.

#### Solution

It’s pretty rare that you do this intentionally, but it could happen from somehow running different logic on the server and in the browser. If you’re seeing warnings like this and you don’t think it’s your fault, it’s much more likely that it’s a bug with `<Suspense/>` or something. Feel free to go ahead and open an [issue](https://github.com/leptos-rs/leptos/issues) or [discussion](https://github.com/leptos-rs/leptos/discussions) on GitHub for help.

#### Solution

You can simply tell the effect to wait a tick before updating the signal, by using something like `request_animation_frame`, which will set a short timeout and then update the signal before the next frame.

```rust
create_effect(move |_| {
    // do something like reading from localStorage
    request_animation_frame(move || set_loaded(true));
});
```

This allows the browser to hydrate with the correct, matching state (`loaded` is `false` when it reaches the view), then immediately update it to `true` once hydration is complete.

### Not all client code can run on the server

Imagine you happily import a dependency like `gloo-net` that you’ve been used to using to make requests in the browser, and use it in a `create_resource` in a server-rendered app.

You’ll probably instantly see the dreaded message

```
panicked at 'cannot call wasm-bindgen imported functions on non-wasm targets'
```

Uh-oh.

But of course this makes sense. We’ve just said that your app needs to run on the client and the server.

#### Solution

There are a few ways to avoid this:

1. Only use libraries that can run on both the server and the client. `reqwest`, for example, works for making HTTP requests in both settings.
2. Use different libraries on the server and the client, and gate them using the `#[cfg]` macro. ([Click here for an example](https://github.com/leptos-rs/leptos/blob/main/examples/hackernews/src/api.rs).)
3. Wrap client-only code in `create_effect`. Because `create_effect` only runs on the client, this can be an effective way to access browser APIs that are not needed for initial rendering.

For example, say that I want to store something in the browser’s `localStorage` whenever a signal changes.

```rust
#[component]
pub fn App() -> impl IntoView {
    use gloo_storage::Storage;
	let storage = gloo_storage::LocalStorage::raw();
	logging::log!("{storage:?}");
}
```

This panics because I can’t access `LocalStorage` during server rendering.

But if I wrap it in an effect...

```rust
#[component]
pub fn App() -> impl IntoView {
    use gloo_storage::Storage;
    create_effect(move |_| {
        let storage = gloo_storage::LocalStorage::raw();
		logging::log!("{storage:?}");
    });
}
```

It’s fine! This will render appropriately on the server, ignoring the client-only code, and then access the storage and log a message on the browser.

### Not all server code can run on the client

WebAssembly running in the browser is a pretty limited environment. You don’t have access to a file-system or to many of the other things the standard library may be used to having. Not every crate can even be compiled to WASM, let alone run in a WASM environment.

In particular, you’ll sometimes see errors about the crate `mio` or missing things from `core`. This is generally a sign that you are trying to compile something to WASM that can’t be compiled to WASM. If you’re adding server-only dependencies, you’ll want to mark them `optional = true` in your `Cargo.toml` and then enable them in the `ssr` feature definition. (Check out one of the template `Cargo.toml` files to see more details.)

You can use `create_effect` to specify that something should only run on the client, and not in the server. Is there a way to specify that something should run only on the server, and not the client?

In fact, there is. The next chapter will cover the topic of server functions in some detail. (In the meantime, you can check out their docs [here](https://docs.rs/leptos_server/latest/leptos_server/index.html).)
