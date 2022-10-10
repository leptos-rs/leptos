# Reactivity

## Signals

A **signal** is a piece of data that may change over time, and notifies other code when it has changed. This is the core primitive of Leptos’s reactive system.

Creating a signal is very simple. You call `create_signal`, passing in the reactive scope and the default value, and receive a tuple containing a `ReadSignal` and a `WriteSignal`.

```rust
let (value, set_value) = create_signal(cx, 0);
```

> If you’ve used signals in Sycamore or Solid, observables in MobX or Knockout, or a similar primitive in reactive library, you probably have a pretty good idea of how signals work in Leptos. If you’re familiar with React, Yew, or Dioxus, you may recognize a similar pattern to their `use_state` hooks.

### `ReadSignal<T>`

The `ReadSignal` half of this tuple allows you to get the current value of the signal. Reading that value in a reactive context automatically subscribes to any further changes. You can access the value by simply calling the `ReadSignal` as a function.

```rust
let (value, set_value) = create_signal(cx, 0);

// calling value() with return the current value of the signal,
// and automatically track changes if you're in a reactive context
assert_eq!(value(), 0);
```

> Here, a **reactive context** means anywhere within an `Effect`. Leptos’s templating system is built on top of its reactive system, so if you’re reading the signal’s value within the template, the template will automatically subscribe to the signal and update exactly the value that needs to change in the DOM.

Calling a `ReadSignal` clones the value it contains. If that’s too expensive, use `ReadSignal::with()` to borrow the value and do whatever you need.

```rust
struct MySuperExpensiveStruct {
    a: String,
    b: StructThatsSuperExpensiveToClone
}
let (value, set_value) = create_signal(cx, MySuperExpensiveStruct::default());

// ❌ this is going to clone the `StructThatsSuperExpensiveToClone` unnecessarily!
let lowercased = move || value().a.to_lowercase();
// ✅ only use what we need
let lowercased = move || value.with(|value| value.to_lowercase());
// 🔥 aaaand there's no need to type "value" three times in a row
let lowercased = move || value.with(String::to_lowercase);
```

### `WriteSignal<T>`

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
