# Lazy Routes

This example demonstrates how to split the WASM bundle that is sent to the client into multiple binaries, which can be lazy-loaded, either independently or in a way that's integrated into the router.

Without code splitting, the entire application is compiled to a monolithic WASM binary, the size of which grows in proportion to the complexity of the application. This means that the time to interactive (TTI) for any page is proportional to the size of the entire application, not only that page.

Code splitting allows you to lazy-load some functions, by splitting off the WASM binary code for certain functions into separate files, which can be downloaded as needed. This minimizes initial TTI for any page, and then amortizes the cost of loading the binary over the lifetime of the application session.

In many cases, this can be done with minimal or no cost.

Lazy loading can be used in two ways, each of which is shown in the example.

## `#[lazy]` macro

`#[lazy]` is an attribute macro that can be used to annotate an `async fn` in order to split its code out into a separate file that will be loaded on demand, when compiled with `cargo leptos --split`.

This has some limitations (for example, it must return concrete types) but can be used for most functions.

## `LazyRoute`

`LazyRoute` is a specialized application of `#[lazy]` that allows you to define an entire route/page of your application as being lazy-loaded.

Creating a lazy route requires you to split the route into two parts:

1. `data()`: A synchronous method that should be used to start loading any async data used by the page, for example by creating a `Resource`
2. `view()`: An async (because lazy-loaded) method that renders the view.

The purpose of splitting these into two parts is to avoid a “waterfall,” in which the browser first waits for a lazy-loaded WASM chunk that defines the page, _then_ makes a second request to the server to load the relevant data. Instead, a `LazyRoute` will begin loading resources created in the `data` method while lazy-loading the component body in the `view`, then render the route.

This means that in many cases, the data loading “hides” the cost of the lazy-loading; i.e., the page needs to wait for the data to load, so the fact that it is waiting concurrently for the lazy-loaded view means that the lazy loading does not cost anything additional in terms of page load time.
