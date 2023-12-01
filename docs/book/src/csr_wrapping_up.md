# Wrapping Up Part 1: Client-Side Rendering

So far, everything we’ve written has been rendered almost entirely in the browser. When we create an app using Trunk, it’s served using a local development server. If you build it for production and deploy it, it’s served by whatever server or CDN you’re using. In either case, what’s served is an HTML page with

1. the URL of your Leptos app, which has been compiled to WebAssembly (WASM)
2. the URL of the JavaScript used to initialize this WASM blob
3. an empty `<body>` element

When the JS and WASM have loaded, Leptos will render your app into the `<body>`. This means that nothing appears on the screen until JS/WASM have loaded and run. This has some drawbacks:

1. It increases load time, as your user’s screen is blank until additional resources have been downloaded.
2. It’s bad for SEO, as load times are longer and the HTML you serve has no meaningful content.
3. It’s broken for users for whom JS/WASM don’t load for some reason (e.g., they’re on a train and just went into a tunnel before WASM finished loading; they’re using an older device that doesn’t support WASM; they have JavaScript or WASM turned off for some reason; etc.)

These downsides apply across the web ecosystem, but especially to WASM apps.

However, depending the on the requirements of your project, you may be fine with these limitations.

If you just want to deploy your Client-Side Rendered website, skip ahead to the chapter on ["Deployment"](https://leptos-rs.github.io/leptos/deployment/index.html) - there, you'll find directions on how best to deploy your Leptos CSR site.


But what do you do if you want to return more than just an empty `<body>` tag in your `index.html` page? Use “Server-Side Rendering”!

Whole books could be (and probably have been) written about this topic, but at its core, it’s really simple: rather than returning an empty `<body>` tag, with SSR, you'll return an initial HTML page that reflects the actual starting state of your app or site, so that while JS/WASM are loading, and until they load, the user can access the plain HTML version.

Part 2 of this book, on Leptos SSR, will cover this topic in some detail!
