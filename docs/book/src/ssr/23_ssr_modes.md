# Async Rendering and SSR “Modes”

Server-rendering a page that uses only synchronous data is pretty simple: You just walk down the component tree, rendering each element to an HTML string. But this is a pretty big caveat: it doesn’t answer the question of what we should do with pages that includes asynchronous data, i.e., the sort of stuff that would be rendered under a `<Suspense/>` node on the client.

When a page loads async data that it needs to render, what should we do? Should we wait for all the async data to load, and then render everything at once? (Let’s call this “async” rendering) Should we go all the way in the opposite direction, just sending the HTML we have immediately down to the client and letting the client load the resources and fill them in? (Let’s call this “synchronous” rendering) Or is there some middle-ground solution that somehow beats them both? (Hint: There is.)

If you’ve ever listened to streaming music or watched a video online, I’m sure you realize that HTTP supports streaming, allowing a single connection to send chunks of data one after another without waiting for the full content to load. You may not realize that browsers are also really good at rendering partial HTML pages. Taken together, this means that you can actually enhance your users’ experience by **streaming HTML**: and this is something that Leptos supports out of the box, with no configuration at all. And there’s actually more than one way to stream HTML: you can stream the chunks of HTML that make up your page in order, like frames of a video, or you can stream them... well, out of order.

Let me say a little more about what I mean.

Leptos supports all the major ways of rendering HTML that includes asynchronous data:

1. [Synchronous Rendering](#synchronous-rendering)
1. [Async Rendering](#async-rendering)
1. [In-Order streaming](#in-order-streaming)
1. [Out-of-Order Streaming](#out-of-order-streaming) (and a partially-blocked variant)

## Synchronous Rendering

1. **Synchronous**: Serve an HTML shell that includes `fallback` for any `<Suspense/>`. Load data on the client using `create_local_resource`, replacing `fallback` once resources are loaded.

- _Pros_: App shell appears very quickly: great TTFB (time to first byte).
- _Cons_
  - Resources load relatively slowly; you need to wait for JS + WASM to load before even making a request.
  - No ability to include data from async resources in the `<title>` or other `<meta>` tags, hurting SEO and things like social media link previews.

If you’re using server-side rendering, the synchronous mode is almost never what you actually want, from a performance perspective. This is because it misses out on an important optimization. If you’re loading async resources during server rendering, you can actually begin loading the data on the server. Rather than waiting for the client to receive the HTML response, then loading its JS + WASM, _then_ realize it needs the resources and begin loading them, server rendering can actually begin loading the resources when the client first makes the response. In this sense, during server rendering an async resource is like a `Future` that begins loading on the server and resolves on the client. As long as the resources are actually serializable, this will always lead to a faster total load time.

> This is why [`create_resource`](https://docs.rs/leptos/latest/leptos/fn.create_resource.html) requires resources data to be serializable by default, and why you need to explicitly use [`create_local_resource`](https://docs.rs/leptos/latest/leptos/fn.create_local_resource.html) for any async data that is not serializable and should therefore only be loaded in the browser itself. Creating a local resource when you could create a serializable resource is always a deoptimization.

## Async Rendering

<video controls>
	<source src="https://github.com/leptos-rs/leptos/blob/main/docs/video/async.mov?raw=true" type="video/mp4">
</video>

2. **`async`**: Load all resources on the server. Wait until all data are loaded, and render HTML in one sweep.

- _Pros_: Better handling for meta tags (because you know async data even before you render the `<head>`). Faster complete load than **synchronous** because async resources begin loading on server.
- _Cons_: Slower load time/TTFB: you need to wait for all async resources to load before displaying anything on the client. The page is totally blank until everything is loaded.

## In-Order Streaming

<video controls>
	<source src="https://github.com/leptos-rs/leptos/blob/main/docs/video/in-order.mov?raw=true" type="video/mp4">
</video>

3. **In-order streaming**: Walk through the component tree, rendering HTML until you hit a `<Suspense/>`. Send down all the HTML you’ve got so far as a chunk in the stream, wait for all the resources accessed under the `<Suspense/>` to load, then render it to HTML and keep walking until you hit another `<Suspense/>` or the end of the page.

- _Pros_: Rather than a blank screen, shows at least _something_ before the data are ready.
- _Cons_
  - Loads the shell more slowly than synchronous rendering (or out-of-order streaming) because it needs to pause at every `<Suspense/>`.
  - Unable to show fallback states for `<Suspense/>`.
  - Can’t begin hydration until the entire page has loaded, so earlier pieces of the page will not be interactive until the suspended chunks have loaded.

## Out-of-Order Streaming

<video controls>
	<source src="https://github.com/leptos-rs/leptos/blob/main/docs/video/out-of-order.mov?raw=true" type="video/mp4">
</video>

4. **Out-of-order streaming**: Like synchronous rendering, serve an HTML shell that includes `fallback` for any `<Suspense/>`. But load data on the **server**, streaming it down to the client as it resolves, and streaming down HTML for `<Suspense/>` nodes, which is swapped in to replace the fallback.

- _Pros_: Combines the best of **synchronous** and **`async`**.
  - Fast initial response/TTFB because it immediately sends the whole synchronous shell
  - Fast total time because resources begin loading on the server.
  - Able to show the fallback loading state and dynamically replace it, instead of showing blank sections for un-loaded data.
- _Cons_: Requires JavaScript to be enabled for suspended fragments to appear in correct order. (This small chunk of JS streamed down in a `<script>` tag alongside the `<template>` tag that contains the rendered `<Suspense/>` fragment, so it does not need to load any additional JS files.)

5. **Partially-blocked streaming**: “Partially-blocked” streaming is useful when you have multiple separate `<Suspense/>` components on the page.  It is triggered by setting `ssr=SsrMode::PartiallyBlocked` on a route, and depending on blocking resources within the view.   If one of the `<Suspense/>` components reads from one or more “blocking resources” (see below), the fallback will not be sent; rather, the server will wait until that `<Suspense/>` has resolved and then replace the fallback with the resolved fragment on the server, which means that it is included in the initial HTML response and appears even if JavaScript is disabled or not supported. Other `<Suspense/>` stream in out of order, similar to the `SsrMode::OutOfOrder` default.

This is useful when you have multiple `<Suspense/>` on the page, and one is more important than the other: think of a blog post and comments, or product information and reviews. It is _not_ useful if there’s only one `<Suspense/>`, or if every `<Suspense/>` reads from blocking resources. In those cases it is a slower form of `async` rendering.

- _Pros_: Works if JavaScript is disabled or not supported on the user’s device.
- _Cons_
  - Slower initial response time than out-of-order.
  - Marginally overall response due to additional work on the server.
  - No fallback state shown.

## Using SSR Modes

Because it offers the best blend of performance characteristics, Leptos defaults to out-of-order streaming. But it’s really simple to opt into these different modes. You do it by adding an `ssr` property onto one or more of your `<Route/>` components, like in the [`ssr_modes` example](https://github.com/leptos-rs/leptos/blob/main/examples/ssr_modes/src/app.rs).

```rust
<Routes>
	// We’ll load the home page with out-of-order streaming and <Suspense/>
	<Route path="" view=HomePage/>

	// We'll load the posts with async rendering, so they can set
	// the title and metadata *after* loading the data
	<Route
		path="/post/:id"
		view=Post
		ssr=SsrMode::Async
	/>
</Routes>
```

For a path that includes multiple nested routes, the most restrictive mode will be used: i.e., if even a single nested route asks for `async` rendering, the whole initial request will be rendered `async`. `async` is the most restricted requirement, followed by in-order, and then out-of-order. (This probably makes sense if you think about it for a few minutes.)

## Blocking Resources

Any Leptos versions later than `0.2.5` (i.e., git main and `0.3.x` or later) introduce a new resource primitive with `create_blocking_resource`. A blocking resource still loads asynchronously like any other `async`/`.await` in Rust; it doesn’t block a server thread or anything. Instead, reading from a blocking resource under a `<Suspense/>` blocks the HTML _stream_ from returning anything, including its initial synchronous shell, until that `<Suspense/>` has resolved.

Now from a performance perspective, this is not ideal. None of the synchronous shell for your page will load until that resource is ready. However, rendering nothing means that you can do things like set the `<title>` or `<meta>` tags in your `<head>` in actual HTML. This sounds a lot like `async` rendering, but there’s one big difference: if you have multiple `<Suspense/>` sections, you can block on _one_ of them but still render a placeholder and then stream in the other.

For example, think about a blog post. For SEO and for social sharing, I definitely want my blog post’s title and metadata in the initial HTML `<head>`. But I really don’t care whether comments have loaded yet or not; I’d like to load those as lazily as possible.

With blocking resources, I can do something like this:

```rust
#[component]
pub fn BlogPost() -> impl IntoView {
	let post_data = create_blocking_resource(/* load blog post */);
	let comment_data = create_resource(/* load blog post */);
	view! {
		<Suspense fallback=|| ()>
			{move || {
				post_data.with(|data| {
					view! {
						<Title text=data.title/>
						<Meta name="description" content=data.excerpt/>
						<article>
							/* render the post content */
						</article>
					}
				})
			}}
		</Suspense>
		<Suspense fallback=|| "Loading comments...">
			/* render comment data here */
		</Suspense>
	}
}
```

The first `<Suspense/>`, with the body of the blog post, will block my HTML stream, because it reads from a blocking resource.  Meta tags and other head elements awaiting the blocking resource will be rendered before the stream is sent.

Combined with the following route definition, which uses `SsrMode::PartiallyBlocked`, the blocking resource will be fully rendered on the server side, making it accessible to users who disable WebAssembly or JavaScript.

```rust
<Routes>
	// We’ll load the home page with out-of-order streaming and <Suspense/>
	<Route path="" view=HomePage/>

	// We'll load the posts with async rendering, so they can set
	// the title and metadata *after* loading the data
	<Route
		path="/post/:id"
		view=Post
		ssr=SsrMode::PartiallyBlocked
	/>
</Routes>
```

The second `<Suspense/>`, with the comments, will not block the stream. Blocking resources gave me exactly the power and granularity I needed to optimize my page for SEO and user experience.
