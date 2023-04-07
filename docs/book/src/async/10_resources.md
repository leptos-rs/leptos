# Loading Data with Resources

A [Resource](https://docs.rs/leptos/latest/leptos/struct.Resource.html) is a reactive data structure that reflects the current state of an asynchronous task, allowing you to integrate asynchronous `Future`s into the synchronous reactive system. Rather than waiting for its data to load with `.await`, you transform the `Future` into a signal that returns `Some(T)` if it has resolved, and `None` if it’s still pending.

You do this by using the [`create_resource`](https://docs.rs/leptos/latest/leptos/fn.create_resource.html) function. This takes two arguments (other than the ubiquitous `cx`):

1. a source signal, which will generate a new `Future` whenever it changes
2. a fetcher function, which takes the data from that signal and returns a `Future`

Here’s an example

```rust
// our source signal: some synchronous, local state
let (count, set_count) = create_signal(cx, 0);

// our resource
let async_data = create_resource(cx,
    count,
    // every time `count` changes, this will run
    |value| async move {
        log!("loading data from API");
        load_data(value).await
    },
);
```

To create a resource that simply runs once, you can pass a non-reactive, empty source signal:

```rust
let once = create_resource(cx, || (), |_| async move { load_data().await });
```

To access the value you can use `.read(cx)` or `.with(cx, |data| /* */)`. These work just like `.get()` and `.with()` on a signal—`read` clones the value and returns it, `with` applies a closure to it—but with two differences

1. For any `Resource<_, T>`, they always return `Option<T>`, not `T`: because it’s always possible that your resource is still loading.
2. They take a `Scope` argument. You’ll see why in the next chapter, on `<Suspense/>`.

So, you can show the current state of a resource in your view:

```rust
let once = create_resource(cx, || (), |_| async move { load_data().await });
view! { cx,
    <h1>"My Data"</h1>
    {move || match once.read(cx) {
        None => view! { cx, <p>"Loading..."</p> }.into_view(cx),
        Some(data) => view! { cx, <ShowData data/> }.into_view(cx)
    }}
}
```

Resources also provide a `refetch()` method that allows you to manually reload the data (for example, in response to a button click) and a `loading()` method that returns a `ReadSignal<bool>` indicating whether the resource is currently loading or not.

[Click to open CodeSandbox.](https://codesandbox.io/p/sandbox/10-async-resources-4z0qt3?file=%2Fsrc%2Fmain.rs&selection=%5B%7B%22endColumn%22%3A1%2C%22endLineNumber%22%3A3%2C%22startColumn%22%3A1%2C%22startLineNumber%22%3A3%7D%5D)

<iframe src="https://codesandbox.io/p/sandbox/10-async-resources-4z0qt3?file=%2Fsrc%2Fmain.rs&selection=%5B%7B%22endColumn%22%3A1%2C%22endLineNumber%22%3A3%2C%22startColumn%22%3A1%2C%22startLineNumber%22%3A3%7D%5D" width="100%" height="1000px" style="max-height: 100vh"></iframe>
