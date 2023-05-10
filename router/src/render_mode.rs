/// Indicates which rendering mode should be used for this route during server-side rendering.
///
/// Leptos supports four different ways to render HTML that contains `async` data loaded
/// under `<Suspense/>`.
/// 1. **Synchronous**: Serve an HTML shell that includes `fallback` for any `Suspense`. Load data on the client, replacing `fallback` once they're loaded.
///     - *Pros*: App shell appears very quickly: great TTFB (time to first byte).
///     - *Cons*: Resources load relatively slowly; you need to wait for JS + Wasm to load before even making a request.
/// 2. **Out-of-order streaming**: Serve an HTML shell that includes `fallback` for any `Suspense`. Load data on the **server**, streaming it down to the client as it resolves, and streaming down HTML for `Suspense` nodes.
///     - *Pros*: Combines the best of **synchronous** and **`async`**, with a very fast shell and resources that begin loading on the server.
///     - *Cons*: Requires JS for suspended fragments to appear in correct order. Weaker meta tag support when it depends on data that's under suspense (has already streamed down `<head>`)
/// 3. **Partially-blocked out-of-order streaming**: Using `create_blocking_resource` with out-of-order streaming still sends fallbacks and relies on JavaScript to fill them in with the fragments. Partially-blocked streaming does this replacement on the server, making for a slower response but requiring no JavaScript to show blocking resources.
///     - *Pros*: Works better if JS is disabled.
///     - *Cons*: Slower initial response because of additional string manipulation on server.
/// 4. **In-order streaming**: Walk through the tree, returning HTML synchronously as in synchronous rendering and out-of-order streaming until you hit a `Suspense`. At that point, wait for all its data to load, then render it, then the rest of the tree.
///     - *Pros*: Does not require JS for HTML to appear in correct order.
///     - *Cons*: Loads the shell more slowly than out-of-order streaming or synchronous rendering because it needs to pause at every `Suspense`. Cannot begin hydration until the entire page has loaded, so earlier pieces
///       of the page will not be interactive until the suspended chunks have loaded.
/// 5. **`async`**: Load all resources on the server. Wait until all data are loaded, and render HTML in one sweep.
///     - *Pros*: Better handling for meta tags (because you know async data even before you render the `<head>`). Faster complete load than **synchronous** because async resources begin loading on server.
///     - *Cons*: Slower load time/TTFB: you need to wait for all async resources to load before displaying anything on the client.
///
/// The mode defaults to out-of-order streaming. For a path that includes multiple nested routes, the most
/// restrictive mode will be used: i.e., if even a single nested route asks for `async` rendering, the whole initial
/// request will be rendered `async`. (`async` is the most restricted requirement, followed by in-order, out-of-order, and synchronous.)
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SsrMode {
    #[default]
    OutOfOrder,
    PartiallyBlocked,
    InOrder,
    Async,
}
