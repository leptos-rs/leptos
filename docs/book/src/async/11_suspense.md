# `<Suspense/>`

In the previous chapter, we showed how you can create a simple loading screen to show some fallback while a resource is loading.

```rust
let (count, set_count) = create_signal(cx, 0);
let a = create_resource(cx, count, |count| async move { load_a(count).await });

view! { cx,
	<h1>"My Data"</h1>
	{move || match once.read(cx) {
		None => view! { cx, <p>"Loading..."</p> }.into_view(cx),
		Some(data) => view! { cx, <ShowData data/> }.into_view(cx)
	}}
}
```

But what if we have two resources, and want to wait for both of them?

```rust
let (count, set_count) = create_signal(cx, 0);
let (count2, set_count2) = create_signal(cx, 0);
let a = create_resource(cx, count, |count| async move { load_a(count).await });
let b = create_resource(cx, count2, |count| async move { load_b(count).await });

view! { cx,
	<h1>"My Data"</h1>
	{move || match (a.read(cx), b.read(cx)) {
		_ => view! { cx, <p>"Loading..."</p> }.into_view(cx),
		(Some(a), Some(b)) => view! { cx,
			<ShowA a/>
			<ShowA b/>
		}.into_view(cx)
	}}
}
```

That’s not _so_ bad, but it’s kind of annoying. What if we could invert the flow of control?

The [`<Suspense/>`](https://docs.rs/leptos/latest/leptos/fn.Suspense.html) component lets us do exactly that. You give it a `fallback` prop and children, one or more of which usually involves reading from a resource. Reading from a resource “under” a `<Suspense/>` (i.e., in one of its children) registers that resource with the `<Suspense/>`. If it’s still waiting for resources to load, it shows the `fallback`. When they’ve all loaded, it shows the children.

```rust
let (count, set_count) = create_signal(cx, 0);
let (count2, set_count2) = create_signal(cx, 0);
let a = create_resource(cx, count, |count| async move { load_a(count).await });
let b = create_resource(cx, count2, |count| async move { load_b(count).await });

view! { cx,
	<h1>"My Data"</h1>
	<Suspense
		fallback=move || view! { cx, <p>"Loading..."</p> }
	>
		<h2>"My Data"</h2>
		<h3>"A"</h3>
		{move || {
			a.read(cx)
				.map(|a| view! { cx, <ShowA a/> })
		}}
		<h3>"B"</h3>
		{move || {
			b.read(cx)
				.map(|b| view! { cx, <ShowB b/> })
		}}
	</Suspense>
}
```

Every time one of the resources is reloading, the `"Loading..."` fallback will show again.

This inversion of the flow of control makes it easier to add or remove individual resources, as you don’t need to handle the matching yourself. It also unlocks some massive performance improvements during server-side rendering, which we’ll talk about during a later chapter.

<iframe src="https://codesandbox.io/p/sandbox/10-async-resources-4z0qt3?file=%2Fsrc%2Fmain.rs&selection=%5B%7B%22endColumn%22%3A1%2C%22endLineNumber%22%3A3%2C%22startColumn%22%3A1%2C%22startLineNumber%22%3A3%7D%5D" width="100%" height="1000px"></iframe>
