# Progressive Enhancement (and Graceful Degradation)

I’ve been driving around Boston for about fifteen years. If you don’t know Boston, let me tell you: Massachusetts has some of the most aggressive drivers(and pedestrians!) in the world. I’ve learned to practice what’s sometimes called “defensive driving”: assuming that someone’s about to swerve in front of you at an intersection when you have the right of way, preparing for a pedestrian to cross into the street at any moment, and driving accordingly.

“Progressive enhancement” is the “defensive driving” of web design. Or really, that’s “graceful degradation,” although they’re two sides of the same coin, or the same process, from two different directions.

**Progressive enhancement**, in this context, means beginning with a simple HTML site or application that works for any user who arrives at your page, and gradually enhancing it with layers of additional features: CSS for styling, JavaScript for interactivity, WebAssembly for Rust-powered interactivity; using particular Web APIs for a richer experience if they’re available and as needed.

**Graceful degradation** means handling failure gracefully when parts of that stack of enhancement *aren’t* available. Here are some sources of failure your users might encounter in your app:
- Their browser doesn’t support WebAssembly because it needs to be updated.
- Their browser can’t support WebAssembly because browser updates are limited to newer OS versions, which can’t be installed on the device. (Looking at you, Apple.)
- They have WASM turned off for security or privacy reasons.
- They have JavaScript turned off for security or privacy reasons.
- JavaScript isn’t supported on their device (for example, some accessibility devices only support HTML browsing)
- The JavaScript (or WASM) never arrived at their device because they walked outside and lost WiFi.
- They stepped onto a subway car after loading the initial page and subsequent navigations can’t load data.
- ... and so on.

How much of your app still works if one of these holds true? Two of them? Three? 

If the answer is something like “95%... okay, then 90%... okay, then 75%,” that’s graceful degradation. If the answer is “my app shows a blank screen unless everything works correctly,” that’s... rapid unscheduled disassembly.

**Graceful degradation is especially important for WASM apps,** because WASM is the newest and least-likely-to-be-supported of the four languages that run in the browser (HTML, CSS, JS, WASM).

Luckily, we’ve got some tools to help.

## Defensive Design

There are a few practices that can help your apps degrade more gracefully:
1. **Server-side rendering.** Without SSR, your app simply doesn’t work without both JS and WASM loading. In some cases this may be appropriate (think internal apps gated behind a login) but in others it’s simply broken.
2. **Native HTML elements.** Use HTML elements that do the things that you want, without additional code: `<a>` for navigation (including to hashes within the page), `<details>` for an accordion, `<form>` to persist information in the URL, etc.
3. **URL-driven state.** The more of your global state is stored in the URL (as a route param or part of the query string), the more of the page can be generated during server rendering and updated by an `<a>` or a `<form>`, which means that not only navigations but state changes can work without JS/WASM.
4. **[`SsrMode::PartiallyBlocked` or `SsrMode::InOrder`](https://docs.rs/leptos_router/latest/leptos_router/enum.SsrMode.html).** Out-of-order streaming requires a small amount of inline JS, but can fail if 1) the connection is broken halfway through the response or 2) the client’s device doesn’t support JS. Async streaming will give a complete HTML page, but only after all resources load. In-order streaming begins showing pieces of the page sooner, in top-down order. “Partially-blocked” SSR builds on out-of-order streaming by replacing `<Suspense/>` fragments that read from blocking resources on the server. This adds marginally to the initial response time (because of the `O(n)` string replacement work), in exchange for a more complete initial HTML response. This can be a good choice for situations in which there’s a clear distinction between “more important” and “less important” content, e.g., blog post vs. comments, or product info vs. reviews. If you choose to block on all the content, you’ve essentially recreated async rendering.
5. **Leaning on `<form>`s.** There’s been a bit of a `<form>` renaissance recently, and it’s no surprise. The ability of a `<form>` to manage complicated `POST` or `GET` requests in an easily-enhanced way makes it a powerful tool for graceful degradation. The example in [the `<Form/>` chapter](../router/20_form.md), for example, would work fine with no JS/WASM: because it uses a `<form method="GET">` to persist state in the URL, it works with pure HTML by making normal HTTP requests and then progressively enhances to use client-side navigations instead.

There’s one final feature of the framework that we haven’t seen yet, and which builds on this characteristic of forms to build powerful applications: the `<ActionForm/>`.