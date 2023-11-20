# Params and Queries

Static paths are useful for distinguishing between different pages, but almost every application wants to pass data through the URL at some point.

There are two ways you can do this:

1. named route **params** like `id` in `/users/:id`
2. named route **queries** like `q` in `/search?q=Foo`

Because of the way URLs are built, you can access the query from _any_ `<Route/>` view. You can access route params from the `<Route/>` that defines them or any of its nested children.

Accessing params and queries is pretty simple with a couple of hooks:

- [`use_query`](https://docs.rs/leptos_router/latest/leptos_router/fn.use_query.html) or [`use_query_map`](https://docs.rs/leptos_router/latest/leptos_router/fn.use_query_map.html)
- [`use_params`](https://docs.rs/leptos_router/latest/leptos_router/fn.use_params.html) or [`use_params_map`](https://docs.rs/leptos_router/latest/leptos_router/fn.use_query_map.html)

Each of these comes with a typed option (`use_query` and `use_params`) and an untyped option (`use_query_map` and `use_params_map`).

The untyped versions hold a simple key-value map. To use the typed versions, derive the [`Params`](https://docs.rs/leptos_router/0.2.3/leptos_router/trait.Params.html) trait on a struct.

> `Params` is a very lightweight trait to convert a flat key-value map of strings into a struct by applying `FromStr` to each field. Because of the flat structure of route params and URL queries, it’s significantly less flexible than something like `serde`; it also adds much less weight to your binary.

```rust
use leptos::*;
use leptos_router::*;

#[derive(Params)]
struct ContactParams {
	id: usize
}

#[derive(Params)]
struct ContactSearch {
	q: String
}
```

> Note: The `Params` derive macro is located at `leptos::Params`, and the `Params` trait is at `leptos_router::Params`. If you avoid using glob imports like `use leptos::*;`, make sure you’re importing the right one for the derive macro.
>
> If you are not using the `nightly` feature, you will get the error
>
> ```
> no function or associated item named `into_param` found for struct `std::string::String` in the current scope
> ```
>
> At the moment, supporting both `T: FromStr` and `Option<T>` for typed params requires a nightly feature. You can fix this by simply changing the struct to use `q: Option<String>` instead of `q: String`.

Now we can use them in a component. Imagine a URL that has both params and a query, like `/contacts/:id?q=Search`.

The typed versions return `Memo<Result<T, _>>`. It’s a Memo so it reacts to changes in the URL. It’s a `Result` because the params or query need to be parsed from the URL, and may or may not be valid.

```rust
let params = use_params::<ContactParams>();
let query = use_query::<ContactSearch>();

// id: || -> usize
let id = move || {
	params.with(|params| {
		params
			.map(|params| params.id)
			.unwrap_or_default()
	})
};
```

The untyped versions return `Memo<ParamsMap>`. Again, it’s memo to react to changes in the URL. [`ParamsMap`](https://docs.rs/leptos_router/0.2.3/leptos_router/struct.ParamsMap.html) behaves a lot like any other map type, with a `.get()` method that returns `Option<&String>`.

```rust
let params = use_params_map();
let query = use_query_map();

// id: || -> Option<String>
let id = move || {
	params.with(|params| params.get("id").cloned())
};
```

This can get a little messy: deriving a signal that wraps an `Option<_>` or `Result<_>` can involve a couple steps. But it’s worth doing this for two reasons:

1. It’s correct, i.e., it forces you to consider the cases, “What if the user doesn’t pass a value for this query field? What if they pass an invalid value?”
2. It’s performant. Specifically, when you navigate between different paths that match the same `<Route/>` with only params or the query changing, you can get fine-grained updates to different parts of your app without rerendering. For example, navigating between different contacts in our contact-list example does a targeted update to the name field (and eventually contact info) without needing to replace or rerender the wrapping `<Contact/>`. This is what fine-grained reactivity is for.

> This is the same example from the previous section. The router is such an integrated system that it makes sense to provide a single example highlighting multiple features, even if we haven’t explained them all yet.

[Click to open CodeSandbox.](https://codesandbox.io/p/sandbox/16-router-0-5-4xp4zz?file=%2Fsrc%2Fmain.rs%3A102%2C2)

<iframe src="https://codesandbox.io/p/sandbox/16-router-0-5-4xp4zz?file=%2Fsrc%2Fmain.rs%3A102%2C2" width="100%" height="1000px" style="max-height: 100vh"></iframe>

<details>
<summary>CodeSandbox Source</summary>

```rust
use leptos::*;
use leptos_router::*;

#[component]
fn App() -> impl IntoView {
    view! {
        <Router>
            <h1>"Contact App"</h1>
            // this <nav> will show on every routes,
            // because it's outside the <Routes/>
            // note: we can just use normal <a> tags
            // and the router will use client-side navigation
            <nav>
                <h2>"Navigation"</h2>
                <a href="/">"Home"</a>
                <a href="/contacts">"Contacts"</a>
            </nav>
            <main>
                <Routes>
                    // / just has an un-nested "Home"
                    <Route path="/" view=|| view! {
                        <h3>"Home"</h3>
                    }/>
                    // /contacts has nested routes
                    <Route
                        path="/contacts"
                        view=ContactList
                      >
                        // if no id specified, fall back
                        <Route path=":id" view=ContactInfo>
                            <Route path="" view=|| view! {
                                <div class="tab">
                                    "(Contact Info)"
                                </div>
                            }/>
                            <Route path="conversations" view=|| view! {
                                <div class="tab">
                                    "(Conversations)"
                                </div>
                            }/>
                        </Route>
                        // if no id specified, fall back
                        <Route path="" view=|| view! {
                            <div class="select-user">
                                "Select a user to view contact info."
                            </div>
                        }/>
                    </Route>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn ContactList() -> impl IntoView {
    view! {
        <div class="contact-list">
            // here's our contact list component itself
            <div class="contact-list-contacts">
                <h3>"Contacts"</h3>
                <A href="alice">"Alice"</A>
                <A href="bob">"Bob"</A>
                <A href="steve">"Steve"</A>
            </div>

            // <Outlet/> will show the nested child route
            // we can position this outlet wherever we want
            // within the layout
            <Outlet/>
        </div>
    }
}

#[component]
fn ContactInfo() -> impl IntoView {
    // we can access the :id param reactively with `use_params_map`
    let params = use_params_map();
    let id = move || params.with(|params| params.get("id").cloned().unwrap_or_default());

    // imagine we're loading data from an API here
    let name = move || match id().as_str() {
        "alice" => "Alice",
        "bob" => "Bob",
        "steve" => "Steve",
        _ => "User not found.",
    };

    view! {
        <div class="contact-info">
            <h4>{name}</h4>
            <div class="tabs">
                <A href="" exact=true>"Contact Info"</A>
                <A href="conversations">"Conversations"</A>
            </div>

            // <Outlet/> here is the tabs that are nested
            // underneath the /contacts/:id route
            <Outlet/>
        </div>
    }
}

fn main() {
    leptos::mount_to_body(App)
}
```

</details>
</preview>
