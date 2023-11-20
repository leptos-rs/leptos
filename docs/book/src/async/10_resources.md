# Loading Data with Resources

A [Resource](https://docs.rs/leptos/latest/leptos/struct.Resource.html) is a reactive data structure that reflects the current state of an asynchronous task, allowing you to integrate asynchronous `Future`s into the synchronous reactive system. Rather than waiting for its data to load with `.await`, you transform the `Future` into a signal that returns `Some(T)` if it has resolved, and `None` if it’s still pending.

You do this by using the [`create_resource`](https://docs.rs/leptos/latest/leptos/fn.create_resource.html) function. This takes two arguments:

1. a source signal, which will generate a new `Future` whenever it changes
2. a fetcher function, which takes the data from that signal and returns a `Future`

Here’s an example

```rust
// our source signal: some synchronous, local state
let (count, set_count) = create_signal(0);

// our resource
let async_data = create_resource(
    count,
    // every time `count` changes, this will run
    |value| async move {
        logging::log!("loading data from API");
        load_data(value).await
    },
);
```

To create a resource that simply runs once, you can pass a non-reactive, empty source signal:

```rust
let once = create_resource(|| (), |_| async move { load_data().await });
```

To access the value you can use `.get()` or `.with(|data| /* */)`. These work just like `.get()` and `.with()` on a signal—`get` clones the value and returns it, `with` applies a closure to it—but for any `Resource<_, T>`, they always return `Option<T>`, not `T`: because it’s always possible that your resource is still loading.

So, you can show the current state of a resource in your view:

```rust
let once = create_resource(|| (), |_| async move { load_data().await });
view! {
    <h1>"My Data"</h1>
    {move || match once.get() {
        None => view! { <p>"Loading..."</p> }.into_view(),
        Some(data) => view! { <ShowData data/> }.into_view()
    }}
}
```

Resources also provide a `refetch()` method that allows you to manually reload the data (for example, in response to a button click) and a `loading()` method that returns a `ReadSignal<bool>` indicating whether the resource is currently loading or not.

[Click to open CodeSandbox.](https://codesandbox.io/p/sandbox/10-resources-0-5-x6h5j6?file=%2Fsrc%2Fmain.rs%3A2%2C3)

<iframe src="https://codesandbox.io/p/sandbox/10-resources-0-5-9jq86q?file=%2Fsrc%2Fmain.rs%3A2%2C3" width="100%" height="1000px" style="max-height: 100vh"></iframe>

<details>
<summary>CodeSandbox Source</summary>

```rust
use gloo_timers::future::TimeoutFuture;
use leptos::*;

// Here we define an async function
// This could be anything: a network request, database read, etc.
// Here, we just multiply a number by 10
async fn load_data(value: i32) -> i32 {
    // fake a one-second delay
    TimeoutFuture::new(1_000).await;
    value * 10
}

#[component]
fn App() -> impl IntoView {
    // this count is our synchronous, local state
    let (count, set_count) = create_signal(0);

    // create_resource takes two arguments after its scope
    let async_data = create_resource(
        // the first is the "source signal"
        count,
        // the second is the loader
        // it takes the source signal's value as its argument
        // and does some async work
        |value| async move { load_data(value).await },
    );
    // whenever the source signal changes, the loader reloads

    // you can also create resources that only load once
    // just return the unit type () from the source signal
    // that doesn't depend on anything: we just load it once
    let stable = create_resource(|| (), |_| async move { load_data(1).await });

    // we can access the resource values with .get()
    // this will reactively return None before the Future has resolved
    // and update to Some(T) when it has resolved
    let async_result = move || {
        async_data
            .get()
            .map(|value| format!("Server returned {value:?}"))
            // This loading state will only show before the first load
            .unwrap_or_else(|| "Loading...".into())
    };

    // the resource's loading() method gives us a
    // signal to indicate whether it's currently loading
    let loading = async_data.loading();
    let is_loading = move || if loading() { "Loading..." } else { "Idle." };

    view! {
        <button
            on:click=move |_| {
                set_count.update(|n| *n += 1);
            }
        >
            "Click me"
        </button>
        <p>
            <code>"stable"</code>": " {move || stable.get()}
        </p>
        <p>
            <code>"count"</code>": " {count}
        </p>
        <p>
            <code>"async_value"</code>": "
            {async_result}
            <br/>
            {is_loading}
        </p>
    }
}

fn main() {
    leptos::mount_to_body(App)
}
```

</details>
</preview>
