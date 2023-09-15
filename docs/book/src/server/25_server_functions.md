# Server Functions

If you’re creating anything beyond a toy app, you’ll need to run code on the server all the time: reading from or writing to a database that only runs on the server, running expensive computations using libraries you don’t want to ship down to the client, accessing APIs that need to be called from the server rather than the client for CORS reasons or because you need a secret API key that’s stored on the server and definitely shouldn’t be shipped down to a user’s browser.

Traditionally, this is done by separating your server and client code, and by setting up something like a REST API or GraphQL API to allow your client to fetch and mutate data on the server. This is fine, but it requires you to write and maintain your code in multiple separate places (client-side code for fetching, server-side functions to run), as well as creating a third thing to manage, which is the API contract between the two.

Leptos is one of a number of modern frameworks that introduce the concept of **server functions**. Server functions have two key characteristics:

1. Server functions are **co-located** with your component code, so that you can organize your work by feature, not by technology. For example, you might have a “dark mode” feature that should persist a user’s dark/light mode preference across sessions, and be applied during server rendering so there’s no flicker. This requires a component that needs to be interactive on the client, and some work to be done on the server (setting a cookie, maybe even storing a user in a database.) Traditionally, this feature might end up being split between two different locations in your code, one in your “frontend” and one in your “backend.” With server functions, you’ll probably just write them both in one `dark_mode.rs` and forget about it.
2. Server functions are **isomorphic**, i.e., they can be called either from the server or the browser. This is done by generating code differently for the two platforms. On the server, a server function simply runs. In the browser, the server function’s body is replaced with a stub that actually makes a fetch request to the server, serializing the arguments into the request and deserializing the return value from the response. But on either end, the function can simply be called: you can create an `add_todo` function that writes to your database, and simply call it from a click handler on a button in the browser!

## Using Server Functions

Actually, I kind of like that example. What would it look like? It’s pretty simple, actually.

```rust
// todo.rs

#[server(AddTodo, "/api")]
pub async fn add_todo(title: String) -> Result<(), ServerFnError> {
    let mut conn = db().await?;

    match sqlx::query("INSERT INTO todos (title, completed) VALUES ($1, false)")
        .bind(title)
        .execute(&mut conn)
        .await
    {
        Ok(_row) => Ok(()),
        Err(e) => Err(ServerFnError::ServerError(e.to_string())),
    }
}

#[component]
pub fn BusyButton() -> impl IntoView {
	view! {
        <button on:click=move |_| {
            spawn_local(async {
                add_todo("So much to do!".to_string()).await;
            });
        }>
            "Add Todo"
        </button>
	}
}
```

You’ll notice a couple things here right away:

- Server functions can use server-only dependencies, like `sqlx`, and can access server-only resources, like our database.
- Server functions are `async`. Even if they only did synchronous work on the server, the function signature would still need to be `async`, because calling them from the browser _must_ be asynchronous.
- Server functions return `Result<T, ServerFnError>`. Again, even if they only do infallible work on the server, this is true, because `ServerFnError`’s variants include the various things that can be wrong during the process of making a network request.
- Server functions can be called from the client. Take a look at our click handler. This is code that will _only ever_ run on the client. But it can call the function `add_todo` (using `spawn_local` to run the `Future`) as if it were an ordinary async function:

```rust
move |_| {
	spawn_local(async {
		add_todo("So much to do!".to_string()).await;
	});
}
```

- Server functions are top-level functions defined with `fn`. Unlike event listeners, derived signals, and most everything else in Leptos, they are not closures! As `fn` calls, they have no access to the reactive state of your app or anything else that is not passed in as an argument. And again, this makes perfect sense: When you make a request to the server, the server doesn’t have access to client state unless you send it explicitly. (Otherwise we’d have to serialize the whole reactive system and send it across the wire with every request, which—while it served classic ASP for a while—is a really bad idea.)
- Server function arguments and return values both need to be serializable with `serde`. Again, hopefully this makes sense: while function arguments in general don’t need to be serialized, calling a server function from the browser means serializing the arguments and sending them over HTTP.

There are a few things to note about the way you define a server function, too.

- Server functions are created by using the [`#[server]` macro](https://docs.rs/leptos_server/latest/leptos_server/index.html#server) to annotate a top-level function, which can be defined anywhere.
- We provide the macro a type name. The type name is used internally as a container to hold, serialize, and deserialize the arguments.
- We provide the macro a path. This is a prefix for the path at which we’ll mount a server function handler on our server. (See examples for [Actix](https://github.com/leptos-rs/leptos/blob/main/examples/todo_app_sqlite/src/main.rs#L44) and [Axum](https://github.com/leptos-rs/leptos/blob/598523cd9d0d775b017cb721e41ebae9349f01e2/examples/todo_app_sqlite_axum/src/main.rs#L51).)
- You’ll need to have `serde` as a dependency with the `derive` featured enabled for the macro to work properly. You can easily add it to `Cargo.toml` with `cargo add serde --features=derive`.

## Server Function URL Prefixes

You can optionally define a specific URL prefix to be used in the definition of the server function.
This is done by providing an optional 2nd argument to the `#[server]` macro.
By default the URL prefix will be `/api`, if not specified.
Here are some examples:

```rust
#[server(AddTodo)]         // will use the default URL prefix of `/api`
#[server(AddTodo, "/foo")] // will use the URL prefix of `/foo`
```

## Server Function Encodings

By default, the server function call is a `POST` request that serializes the arguments as URL-encoded form data in the body of the request. (This means that server functions can be called from HTML forms, which we’ll see in a future chapter.) But there are a few other methods supported. Optionally, we can provide another argument to the `#[server]` macro to specify an alternate encoding:

```rust
#[server(AddTodo, "/api", "Url")]
#[server(AddTodo, "/api", "GetJson")]
#[server(AddTodo, "/api", "Cbor")]
#[server(AddTodo, "/api", "GetCbor")]
```

The four options use different combinations of HTTP verbs and encoding methods:

| Name              | Method | Request     | Response |
| ----------------- | ------ | ----------- | -------- |
| **Url** (default) | POST   | URL encoded | JSON     |
| **GetJson**       | GET    | URL encoded | JSON     |
| **Cbor**          | POST   | CBOR        | CBOR     |
| **GetCbor**       | GET    | URL encoded | CBOR     |

In other words, you have two choices:

- `GET` or `POST`? This has implications for things like browser or CDN caching; while `POST` requests should not be cached, `GET` requests can be.
- Plain text (arguments sent with URL/form encoding, results sent as JSON) or a binary format (CBOR, encoded as a base64 string)?

**But remember**: Leptos will handle all the details of this encoding and decoding for you. When you use a server function, it looks just like calling any other asynchronous function!

> **Why not `PUT` or `DELETE`? Why URL/form encoding, and not JSON?**
>
> These are reasonable questions. Much of the web is built on REST API patterns that encourage the use of semantic HTTP methods like `DELETE` to delete an item from a database, and many devs are accustomed to sending data to APIs in the JSON format.
>
> The reason we use `POST` or `GET` with URL-encoded data by default is the `<form>` support. For better or for worse, HTML forms don’t support `PUT` or `DELETE`, and they don’t support sending JSON. This means that if you use anything but a `GET` or `POST` request with URL-encoded data, it can only work once WASM has loaded. As we’ll see [in a later chapter](../progressive_enhancement), this isn’t always a great idea.
>
> The CBOR encoding is suported for historical reasons; an earlier version of server functions used a URL encoding that didn’t support nested objects like structs or vectors as server function arguments, which CBOR did. But note that the CBOR forms encounter the same issue as `PUT`, `DELETE`, or JSON: they do not degrade gracefully if the WASM version of your app is not available.


## Server Functions Endpoint Paths

By default, a unique path will be generated. You can optionally define a specific endpoint path to be used in the URL. This is done by providing an optional 4th argument to the `#[server]` macro. Leptos will generate the complete path by concatenating the URL prefix (2nd argument) and the endpoint path (4th argument).
For example,

```rust
#[server(MyServerFnType, "/api", "Url", "hello")]
```
will generate a server function endpoint at `/api/hello` that accepts a POST request.

> **Can I use the same server function endpoint path with multiple encodings?**
>
> No. Different server functions must have unique paths. The `#[server]` macro automatically generates unique paths, but you need to be careful if you choose to specify the complete path manually, as the server looks up server functions by their path.

## An Important Note on Security

Server functions are a cool technology, but it’s very important to remember. **Server functions are not magic; they’re syntax sugar for defining a public API.** The _body_ of a server function is never made public; it’s just part of your server binary. But the server function is a publicly accessible API endpoint, and it’s return value is just a JSON or similar blob. You should _never_ return something sensitive from a server function.

## Integrating Server Functions with Leptos

So far, everything I’ve said is actually framework agnostic. (And in fact, the Leptos server function crate has been integrated into Dioxus as well!) Server functions are simply a way of defining a function-like RPC call that leans on Web standards like HTTP requests and URL encoding.

But in a way, they also provide the last missing primitive in our story so far. Because a server function is just a plain Rust async function, it integrates perfectly with the async Leptos primitives we discussed [earlier](https://leptos-rs.github.io/leptos/async/index.html). So you can easily integrate your server functions with the rest of your applications:

- Create **resources** that call the server function to load data from the server
- Read these resources under `<Suspense/>` or `<Transition/>` to enable streaming SSR and fallback states while data loads.
- Create **actions** that call the server function to mutate data on the server

The final section of this book will make this a little more concrete by introducing patterns that use progressively-enhanced HTML forms to run these server actions.

But in the next few chapters, we’ll actually take a look at some of the details of what you might want to do with your server functions, including the best ways to integrate with the powerful extractors provided by the Actix and Axum server frameworks.
