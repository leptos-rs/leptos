# Leptos Gotchas: Common Bugs

This document is intended as a running list of common issues, with example code and solutions.

## Reactivity

### Avoid writing to a signal from an effect

**Issue**: Sometimes you want to update a reactive signal in a way that depends on another signal.

```rust
let (a, set_a) = create_signal(cx, 0);
let (b, set_b) = create_signal(cx, false);

create_effect(cx, move |_| {
	if a() > 5 {
		set_b(true);
	}
});
```

This creates an inefficient chain of updates, and can easily lead to infinite loops in more complex applications.

**Solution**: Follow the rule, _What can be derived, should be derived._ In this case, this has the benefit of massively reducing the code size, too!

```rust
let (a, set_a) = create_signal(cx, 0);
let b = move || a () > 5;
```

## Templates and the DOM

### `<input value=...>` doesn't update or stops updating

Many DOM attributes can be updated either by setting an attribute on the DOM node, or by setting an object property directly on it. In general, `setAttribute()` stops working once the property has been set.

This means that in practice, attributes like `value` or `checked` on an `<input/>` element only update the _default_ value for the `<input/>`. If you want to reactively update the value, you should use `prop:value` instead to set the `value` property.

```rust
let (a, set_a) = create_signal(cx, "Starting value".to_string());
let on_input = move |ev| set_a(event_target_value(&ev));

view! {
	cx,
	// ❌ reactivity doesn't work as expected: typing only updates the default
	//    of each input, so if you start typing in the second input, it won't
	//    update the first one
	<input value=a on:input=on_input />
	<input value=a on:input=on_input />
}
```

```rust
let (a, set_a) = create_signal(cx, "Starting value".to_string());
let on_input = move |ev| set_a(event_target_value(&ev));

view! {
	cx,
	// ✅ works as intended by setting the value *property*
	<input prop:value=a on:input=on_input />
	<input prop:value=a on:input=on_input />
}
```

## Build configuration

### Cargo feature resolution in workspaces

A new [version](https://doc.rust-lang.org/cargo/reference/resolver.html#resolver-versions) of Cargo's feature resolver was introduced for the 2021 edition of Rust.
For single crate projects it will select a resolver version based on the Rust edition in `Cargo.toml`. As there is no Rust edition present for `Cargo.toml` in a workspace, Cargo will default to the pre 2021 edition resolver.
This can cause issues resulting in non WASM compatible code being built for a WASM target. Seeing `mio` failing to build is often a sign that none WASM compatible code is being included in the build.

The resolver version can be set in the workspace `Cargo.toml` to remedy this issue.

```toml
[workspace]
members = ["member1", "member2"]
resolver = "2"
```
