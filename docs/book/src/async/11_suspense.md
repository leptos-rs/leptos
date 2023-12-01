# `<Suspense/>`

In the previous chapter, we showed how you can create a simple loading screen to show some fallback while a resource is loading.

```rust
let (count, set_count) = create_signal(0);
let once = create_resource(count, |count| async move { load_a(count).await });

view! {
    <h1>"My Data"</h1>
    {move || match once.get() {
        None => view! { <p>"Loading..."</p> }.into_view(),
        Some(data) => view! { <ShowData data/> }.into_view()
    }}
}
```

But what if we have two resources, and want to wait for both of them?

```rust
let (count, set_count) = create_signal(0);
let (count2, set_count2) = create_signal(0);
let a = create_resource(count, |count| async move { load_a(count).await });
let b = create_resource(count2, |count| async move { load_b(count).await });

view! {
    <h1>"My Data"</h1>
    {move || match (a.get(), b.get()) {
        (Some(a), Some(b)) => view! {
            <ShowA a/>
            <ShowA b/>
        }.into_view(),
        _ => view! { <p>"Loading..."</p> }.into_view()
    }}
}
```

That’s not _so_ bad, but it’s kind of annoying. What if we could invert the flow of control?

The [`<Suspense/>`](https://docs.rs/leptos/latest/leptos/fn.Suspense.html) component lets us do exactly that. You give it a `fallback` prop and children, one or more of which usually involves reading from a resource. Reading from a resource “under” a `<Suspense/>` (i.e., in one of its children) registers that resource with the `<Suspense/>`. If it’s still waiting for resources to load, it shows the `fallback`. When they’ve all loaded, it shows the children.

```rust
let (count, set_count) = create_signal(0);
let (count2, set_count2) = create_signal(0);
let a = create_resource(count, |count| async move { load_a(count).await });
let b = create_resource(count2, |count| async move { load_b(count).await });

view! {
    <h1>"My Data"</h1>
    <Suspense
        fallback=move || view! { <p>"Loading..."</p> }
    >
        <h2>"My Data"</h2>
        <h3>"A"</h3>
        {move || {
            a.get()
                .map(|a| view! { <ShowA a/> })
        }}
        <h3>"B"</h3>
        {move || {
            b.get()
                .map(|b| view! { <ShowB b/> })
        }}
    </Suspense>
}
```

Every time one of the resources is reloading, the `"Loading..."` fallback will show again.

This inversion of the flow of control makes it easier to add or remove individual resources, as you don’t need to handle the matching yourself. It also unlocks some massive performance improvements during server-side rendering, which we’ll talk about during a later chapter.

## `<Await/>`

In you’re simply trying to wait for some `Future` to resolve before rendering, you may find the `<Await/>` component helpful in reducing boilerplate. `<Await/>` essentially combines a resource with the source argument `|| ()` with a `<Suspense/>` with no fallback.

In other words:

1. It only polls the `Future` once, and does not respond to any reactive changes.
2. It does not render anything until the `Future` resolves.
3. After the `Future` resolves, it binds its data to whatever variable name you choose and then renders its children with that variable in scope.

```rust
async fn fetch_monkeys(monkey: i32) -> i32 {
    // maybe this didn't need to be async
    monkey * 2
}
view! {
    <Await
        // `future` provides the `Future` to be resolved
        future=|| fetch_monkeys(3)
        // the data is bound to whatever variable name you provide
        let:data
    >
        // you receive the data by reference and can use it in your view here
        <p>{*data} " little monkeys, jumping on the bed."</p>
    </Await>
}
```

[Click to open CodeSandbox.](https://codesandbox.io/p/sandbox/11-suspense-0-5-qzpgqs?file=%2Fsrc%2Fmain.rs%3A1%2C1)

<iframe src="https://codesandbox.io/p/sandbox/11-suspense-0-5-qzpgqs?file=%2Fsrc%2Fmain.rs%3A1%2C1" width="100%" height="1000px" style="max-height: 100vh"></iframe>

<details>
<summary>CodeSandbox Source</summary>

```rust
use gloo_timers::future::TimeoutFuture;
use leptos::*;

async fn important_api_call(name: String) -> String {
    TimeoutFuture::new(1_000).await;
    name.to_ascii_uppercase()
}

#[component]
fn App() -> impl IntoView {
    let (name, set_name) = create_signal("Bill".to_string());

    // this will reload every time `name` changes
    let async_data = create_resource(

        name,
        |name| async move { important_api_call(name).await },
    );

    view! {
        <input
            on:input=move |ev| {
                set_name(event_target_value(&ev));
            }
            prop:value=name
        />
        <p><code>"name:"</code> {name}</p>
        <Suspense
            // the fallback will show whenever a resource
            // read "under" the suspense is loading
            fallback=move || view! { <p>"Loading..."</p> }
        >
            // the children will be rendered once initially,
            // and then whenever any resources has been resolved
            <p>
                "Your shouting name is "
                {move || async_data.get()}
            </p>
        </Suspense>
    }
}

fn main() {
    leptos::mount_to_body(App)
}
```

</details>
</preview>
