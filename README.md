# leptos

A full-stack, isomorphic Rust web framework leveraging fine-grained reactivity to build declarative user interfaces.

** NOTE: This README is a work in progress and is currently incomplete.**

```rust
use leptos::*;

pub fn simple_counter(cx: Scope) -> Element {
	let (value, set_value) = create_signal(cx, 0);

	let clear = move |_| set_value(0);
	let decrement = move |_| set_value.update(|value| *value -= 1);
	let increment = move |_| set_value.update(|value| *value += 1);

    view! {
        <div>
            <button on:click=clear>"Clear"</button>
            <button on:click=decrement>"-1"</button>
            <span>"Value: " {move || value().to_string()} "!"</span>
            <button on:click=increment>"+1"</button>
        </div>
    }
}
```

## Concepts

### Signals

A **signal** is a piece of data that may change over time, and notifies other code when it has changed. This is the core primitive of Leptos’s reactive system.

Creating a signal is very simple. You call `create_signal`, passing in the reactive scope and the default value, and receive a tuple containing a `ReadSignal` and a `WriteSignal`.

```rust
let (value, set_value) = create_signal(cx, 0);
```

> If you’ve used signals in Sycamore or Solid, observables in MobX or Knockout, or a similar primitive in reactive library, you probably have a pretty good idea of how signals work in Leptos. If you’re familiar with React, Yew, or Dioxus, you may recognize a similar pattern to their `use_state` hooks.

#### `ReadSignal<T>`

The `ReadSignal` half of this tuple allows you to get the current value of the signal. Reading that value in a reactive context automatically subscribes to any further changes. You can access the value by calling `ReadSignal::get()` or, more idiomatically, simply calling the `ReadSignal` as a function.

```rust
let (value, set_value) = create_signal(cx, 0);
// value.get() will return the current value, and subscribe if you’re in a reactive context
assert_eq!(value.get(), 0);
// ✅ best practice: simply call value() as a short-hand
assert_eq!(value(), 0);
```

> Here, a **reactive context** means anywhere within an `Effect`. Leptos’s templating system is built on top of its reactive system, so if you’re reading the signal’s value within the template, the template will automatically subscribe to the signal and update exactly the value that needs to change in the DOM.

#### `WriteSignal<T>`

The `WriteSignal` half of this tuple allows you to update the value of the signal, which will automatically notify anything that’s listening to the value that something has changed. If you simply call the `WriteSignal` as a function, its value will be set to the argument you pass. If you want to mutate the value in place instead of replacing it, you can call `WriteSignal::update` instead.

```rust
// often you just want to replace the value
let (value, set_value) = create_signal(cx, 0);
set_value(1);
assert_eq!(value(), 1);

// sometimes you want to mutate something in place, like a Vec. Just call update()
let (items, set_items) = create_signal(cx, vec![0]);
set_items.update(|items: &mut Vec<i32>| items.push(1));
assert_eq!(items(), vec![1]);
```

> Under the hood, `set_value(1)` is just syntactic sugar for `set_value.update(|n| *n = 1)`.
