# Parent-Child Communication

You can think of your application as a nested tree of components. Each component
handles its own local state and manages a section of the user interface, so
components tend to be relatively self-contained.

Sometimes, though, you’ll want to communicate between a parent component and its
child. For example, imagine you’ve defined a `<FancyButton/>` component that adds
some styling, logging, or something else to a `<button/>`. You want to use a
`<FancyButton/>` in your `<App/>` component. But how can you communicate between
the two?

It’s easy to communicate state from a parent component to a child component. We
covered some of this in the material on [components and props](./03_components.md).
Basically if you want the parent to communicate to the child, you can pass a
[`ReadSignal`](https://docs.rs/leptos/latest/leptos/struct.ReadSignal.html), a
[`Signal`](https://docs.rs/leptos/latest/leptos/struct.Signal.html), or even a
[`MaybeSignal`](https://docs.rs/leptos/latest/leptos/struct.MaybeSignal.html) as a prop.

But what about the other direction? How can a child send notifications about events
or state changes back up to the parent?

There are four basic patterns of parent-child communication in Leptos.

## 1. Pass a [`WriteSignal`](https://docs.rs/leptos/latest/leptos/struct.WriteSignal.html)

One approach is simply to pass a `WriteSignal` from the parent down to the child, and update
it in the child. This lets you manipulate the local state of the parent from the child.

```rust
#[component]
pub fn App(cx: Scope) -> impl IntoView {
	let (toggled, set_toggled) = create_signal(cx, false);
	view! { cx,
		<p>"Toggled? " {toggled}</p>
		<ButtonA setter=set_toggled/>
	}
}

#[component]
pub fn ButtonA(cx: Scope, setter: WriteSignal<bool>) -> impl IntoView {
    view! {
        cx,
        <button
            on:click=move |_| setter.update(|value| *value = !*value)
        >
            "Toggle Red"
        </button>
    }
}
```

This pattern is simple, but you should be careful:
