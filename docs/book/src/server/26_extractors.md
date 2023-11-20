# Extractors

The server functions we looked at in the last chapter showed how to run code on the server, and integrate it with the user interface you’re rendering in the browser. But they didn’t show you much about how to actually use your server to its full potential.

## Server Frameworks

We call Leptos a “full-stack” framework, but “full-stack” is always a misnomer (after all, it never means everything from the browser to your power company.) For us, “full stack” means that your Leptos app can run in the browser, and can run on the server, and can integrate the two, drawing together the unique features available in each; as we’ve seen in the book so far, a button click on the browser can drive a database read on the server, both written in the same Rust module. But Leptos itself doesn’t provide the server (or the database, or the operating system, or the firmware, or the electrical cables...)

Instead, Leptos provides integrations for the two most popular Rust web server frameworks, Actix Web ([`leptos_actix`](https://docs.rs/leptos_actix/latest/leptos_actix/)) and Axum ([`leptos_axum`](https://docs.rs/leptos_axum/latest/leptos_axum/)). We’ve built integrations with each server’s router so that you can simply plug your Leptos app into an existing server with `.leptos_routes()`, and easily handle server function calls.

> If you haven’t seen our [Actix](https://github.com/leptos-rs/start) and [Axum](https://github.com/leptos-rs/start-axum) templates, now’s a good time to check them out.

## Using Extractors

Both Actix and Axum handlers are built on the same powerful idea of **extractors**. Extractors “extract” typed data from an HTTP request, allowing you to access server-specific data easily.

Leptos provides `extract` helper functions to let you use these extractors directly in your server functions, with a convenient syntax very similar to handlers for each framework.

### Actix Extractors

The [`extract` function in `leptos_actix`](https://docs.rs/leptos_actix/latest/leptos_actix/fn.extract.html) takes a handler function as its argument. The handler follows similar rules to an Actix handler: it is an async function that receives arguments that will be extracted from the request and returns some value. The handler function receives that extracted data as its arguments, and can do further `async` work on them inside the body of the `async move` block. It returns whatever value you return back out into the server function.

```rust

#[server(ActixExtract, "/api")]
pub async fn actix_extract() -> Result<String, ServerFnError> {
	use leptos_actix::extract;
    use actix_web::dev::ConnectionInfo;
    use actix_web::web::{Data, Query};

    extract(
        |search: Query<Search>, connection: ConnectionInfo| async move {
            format!(
                "search = {}\nconnection = {:?}",
                search.q,
                connection
            )
        },
    )
    .await
}
```

## Axum Extractors

The syntax for the [`leptos_axum::extract`](https://docs.rs/leptos_axum/latest/leptos_axum/fn.extract.html) function is very similar. (**Note**: This is available on the git main branch, but has not been released as of writing.) Note that Axum extractors return a `Result`, so you’ll need to add something to handle the error case.

```rust
#[server(AxumExtract, "/api")]
pub async fn axum_extract() -> Result<String, ServerFnError> {
    use axum::{extract::Query, http::Method};
    use leptos_axum::extract;

    extract(|method: Method, res: Query<MyQuery>| async move {
            format!("{method:?} and {}", res.q)
        },
    )
    .await
    .map_err(|e| ServerFnError::ServerError("Could not extract method and query...".to_string()))
}
```

These are relatively simple examples accessing basic data from the server. But you can use extractors to access things like headers, cookies, database connection pools, and more, using the exact same `extract()` pattern.

The Axum `extract` function only supports extractors for which the state is `()`. If you need an extractor that uses `State`, you should use [`extract_with_state`](https://docs.rs/leptos_axum/latest/leptos_axum/fn.extract_with_state.html). This requires you to provide the state. You can do this by extending the existing `LeptosOptions` state using the Axum `FromRef` pattern, which providing the state as context during render and server functions with custom handlers.

```rust
use axum::extract::FromRef;

/// Derive FromRef to allow multiple items in state, using Axum’s
/// SubStates pattern.
#[derive(FromRef, Debug, Clone)]
pub struct AppState{
    pub leptos_options: LeptosOptions,
    pub pool: SqlitePool
}
```

[Click here for an example of providing context in custom handlers](https://github.com/leptos-rs/leptos/blob/19ea6fae6aec2a493d79cc86612622d219e6eebb/examples/session_auth_axum/src/main.rs#L24-L44).

## A Note about Data-Loading Patterns

Because Actix and (especially) Axum are built on the idea of a single round-trip HTTP request and response, you typically run extractors near the “top” of your application (i.e., before you start rendering) and use the extracted data to determine how that should be rendered. Before you render a `<button>`, you load all the data your app could need. And any given route handler needs to know all the data that will need to be extracted by that route.

But Leptos integrates both the client and the server, and it’s important to be able to refresh small pieces of your UI with new data from the server without forcing a full reload of all the data. So Leptos likes to push data loading “down” in your application, as far towards the leaves of your user interface as possible. When you click a `<button>`, it can refresh just the data it needs. This is exactly what server functions are for: they give you granular access to data to be loaded and reloaded.

The `extract()` functions let you combine both models by using extractors in your server functions. You get access to the full power of route extractors, while decentralizing knowledge of what needs to be extracted down to your individual components. This makes it easier to refactor and reorganize routes: you don’t need to specify all the data a route needs up front.
