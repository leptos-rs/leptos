# Leptos Gotchas: Common Bugs

This document is intended as a running list of common issues, with example code and solutions.

## Reactivity

### Avoid writing to a signal from an effect

**Issue**: Sometimes you want to update a reactive signal in a way that depends on another signal.

```rust
let (a, set_a) = create_signal(0);
let (b, set_b) = create_signal(false);

create_effect(move |_| {
	if a() > 5 {
		set_b(true);
	}
});
```

This creates an inefficient chain of updates, and can easily lead to infinite loops in more complex applications.

**Solution**: Follow the rule, _What can be derived, should be derived._ In this case, this has the benefit of massively reducing the code size, too!

```rust
let (a, set_a) = create_signal(0);
let b = move || a () > 5;
```

### Nested signal updates/reads triggering panic

Sometimes you have nested signals: for example, hash-map that can change over time, each of whose values can also change over time:

```rust
#[component]
pub fn App() -> impl IntoView {
    let resources = create_rw_signal(HashMap::new());

    let update = move |id: usize| {
        resources.update(|resources| {
            resources
                .entry(id)
                .or_insert_with(|| create_rw_signal(0))
                .update(|amount| *amount += 1)
        })
    };

    view! {
        <div>
            <pre>{move || format!("{:#?}", resources.get().into_iter().map(|(id, resource)| (id, resource.get())).collect::<Vec<_>>())}</pre>
            <button on:click=move |_| update(1)>"+"</button>
        </div>
    }
}
```

Clicking the button twice will cause a panic, because of the nested signal _read_. Calling the `update` function on `resources` immediately takes out a mutable borrow on `resources`, then updates the `resource` signal—which re-runs the effect that reads from the signals, which tries to immutably access `resources` and panics. It's the nested update here which causes a problem, because the inner update triggers and effect that tries to read both signals while the outer is still updating.

You can fix this fairly easily by using the [`batch()`](https://docs.rs/leptos/latest/leptos/fn.batch.html) method:

```rust
    let update = move |id: usize| {
        batch(move || {
            resources.update(|resources| {
                resources
                    .entry(id)
                    .or_insert_with(|| create_rw_signal(0))
                    .update(|amount| *amount += 1)
            })
        });
    };
```

This delays running any effects until after both updates are made, preventing the conflict entirely without requiring any other restructuring.

## Templates and the DOM

### `<input value=...>` doesn't update or stops updating

Many DOM attributes can be updated either by setting an attribute on the DOM node, or by setting an object property directly on it. In general, `setAttribute()` stops working once the property has been set.

This means that in practice, attributes like `value` or `checked` on an `<input/>` element only update the _default_ value for the `<input/>`. If you want to reactively update the value, you should use `prop:value` instead to set the `value` property.

```rust
let (a, set_a) = create_signal("Starting value".to_string());
let on_input = move |ev| set_a(event_target_value(&ev));

view! {

	// ❌ reactivity doesn't work as expected: typing only updates the default
	//    of each input, so if you start typing in the second input, it won't
	//    update the first one
	<input value=a on:input=on_input />
	<input value=a on:input=on_input />
}
```

```rust
let (a, set_a) = create_signal("Starting value".to_string());
let on_input = move |ev| set_a(event_target_value(&ev));

view! {

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
