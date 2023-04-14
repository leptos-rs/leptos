# The Life of a Page Load

Before we get into the weeds it might be helpful to have a higher-level overview. What exactly happens between the moment you type in the URL of a server-rendered Leptos app, and the moment you click a button and a counter increases?

I’m assuming some basic knowledge of how the Internet works here, and won’t get into the weeds about HTTP or whatever. Instead, I’ll try to show how different parts of the Leptos APIs map onto each part of the process.

This description also starts from the premise that your app is being compiled for two separate targets:

1. A server version, often running on Actix or Axum, compiled with the Leptos `ssr` feature
2. A browser version, compiled to WebAssembly (WASM) with the Leptos `hydrate` feature

The [`cargo-leptos`](https://github.com/leptos-rs/cargo-leptos) build tool exists to coordinate the process of compiling your app for these two different targets.

## On the Server

- Your browser makes a `GET` request for that URL to your server. At this point, the browser knows almost nothing about the page that’s going to be rendered. (The question “How does the browser know where to ask for the page?” is an interesting one, but out of the scope of this tutorial!)
- The server receives that request, and checks whether it has a way to handle a `GET` request at that path. This is what the `.leptos_routes()` methods in [`leptos_axum`](https://docs.rs/leptos_axum/0.2.5/leptos_axum/trait.LeptosRoutes.html) and [`leptos_actix`](https://docs.rs/leptos_actix/0.2.5/leptos_actix/trait.LeptosRoutes.html) are for. When the server starts up, these methods walk over the routing structure you provide in `<Routes/>`, generating a list of all possible routes your app can handle and telling the server’s router “for each of these routes, if you get a request... hand it off to Leptos.”
- The server sees that this route can be handled by Leptos. So it renders your root component (often called something like `<App/>`), providing it with the URL that’s being requested and some other data like the HTTP headers and request metadata.
- Your application runs once on the server, building up an HTML version of the component tree that will be rendered at that route. (There’s more to be said here about resources and `<Suspense/>` in the next chapter.)
- The server returns this HTML page, also injecting information on how to load the version of your app that has been compiled to WASM so that it can run in the browser.

> The HTML page that’s returned is essentially your app, “dehydrated” or “freeze-dried”: it is HTML without any of the reactivity or event listeners you’ve added. The browser will “rehydrate” this HTML page by adding the reactive system and attaching event listeners to that server-rendered HTML. Hence the two feature flags that apply to the two halves of this process: `ssr` on the server for “server-side rendering”, and `hydrate` in the browser for that process of rehydration.

## In the Browser

- The browser receives this HTML page from the server. It immediately goes back to the server to begin loading the JS and WASM necessary to run the interactive, client side version of the app.
- In the meantime, it renders the HTML version.
- When the WASM version has reloaded, it does the same route-matching process that the server did. Because the `<Routes/>` component is identical on the server and in the client, the browser version will read the URL and render the same page that was already returned by the server.
- During this initial “hydration” phase, the WASM version of your app doesn’t re-create the DOM nodes that make up your application. Instead, it walks over the existing HTML tree, “picking up” existing elements and adding the necessary interactivity.

> Note that there are some trade-offs here. Before this hydration process is complete, the page will _appear_ interactive but won’t actually respond to interactions. For example, if you have a counter button and click it before WASM has loaded, the count will not increment, because the necessary event listeners and reactivity have not been added yet. We’ll look at some ways to build in “graceful degradation” in future chapters.

## Client-Side Navigation

The next step is very important. Imagine that the user now clicks a link to navigate to another page in your application.

The browser will _not_ make another round trip to the server, reloading the full page as it would for navigating between plain HTML pages or an application that uses server rendering (for example with PHP) but without a client-side half.

Instead, the WASM version of your app will load the new page, right there in the browser, without requesting another page from the server. Essentially, your app upgrades itself from a server-loaded “multi-page app” into a browser-rendered “single-page app.” This yields the best of both worlds: a fast initial load time due to the server-rendered HTML, and fast secondary navigations because of the client-side routing.

Some of what will be described in the following chapters—like the interactions between server functions, resources, and `<Suspense/>`—may seem overly complicated. You might find yourself asking, “If my page is being rendered to HTML on the server, why can’t I just `.await` this on the server? If I can just call library X in a server function, why can’t I call it in my component?” The reason is pretty simple: to enable the upgrade from server rendering to client rendering, everything in your application must be able to run either on the server or in the browser.

This is not the only way to create a website or web framework, of course. But it’s the most common way, and we happen to think it’s quite a good way, to create the smoothest possible experience for your users.
