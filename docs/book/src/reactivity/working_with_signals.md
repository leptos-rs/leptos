# Working with Signals

So far we’ve used some simple examples of [`create_signal`](https://docs.rs/leptos/latest/leptos/fn.create_signal.html), which returns a [`ReadSignal`](https://docs.rs/leptos/latest/leptos/struct.ReadSignal.html) getter and a [`WriteSignal`](https://docs.rs/leptos/latest/leptos/struct.WriteSignal.html) setter.

There are four basic signal operations:

1. [`.get()`](https://docs.rs/leptos/latest/leptos/struct.ReadSignal.html#impl-SignalGet%3CT%3E-for-ReadSignal%3CT%3E) clones the current value of the signal and tracks any future changes to the value reactively.
2. [`.with()`](https://docs.rs/leptos/latest/leptos/struct.ReadSignal.html#impl-SignalWith%3CT%3E-for-ReadSignal%3CT%3E) takes a function, which receives the current value of the signal by reference (`&T`), and tracks any future changes.
3. [`.set()`](https://docs.rs/leptos/latest/leptos/struct.WriteSignal.html#impl-SignalSet%3CT%3E-for-WriteSignal%3CT%3E) replaces the current value of the signal and notifies any subscribers that they need to update.
4. [`.update()`](https://docs.rs/leptos/latest/leptos/struct.WriteSignal.html#impl-SignalUpdate%3CT%3E-for-WriteSignal%3CT%3E) takes a function, which receives a mutable reference to the current value of the signal (`&T`), and notifies any subscribers that they need to update. (`.update()` doesn’t return the value returned by the closure, but you can use [`.try_update()`](https://docs.rs/leptos/latest/leptos/trait.SignalUpdate.html#tymethod.try_update) if you need to; for example, if you’re removing an item from a `Vec<_>` and want the removed item.)

Calling a `ReadSignal` as a function is syntax sugar for `.get()`. Calling a `WriteSignal` as a function is syntax sugar for `.set()`. So

```rust
let (count, set_count) = create_signal(cx, 0);
set_count(1);
log!(count());
```

is the same as

```rust
let (count, set_count) = create_signal(cx, 0);
set_count.set(1);
log!(count.get());
```

You might notice that `.get()` and `.set()` can be implemented in terms of `.with()` and `.update()`. In other words, `count.get()` is identical with `count.with(|n| n.clone())`, and `count.set(1)` is implemented by doing `count.update(|n| *n = 1)`.

But of course, `.get()` and `.set()` (or the plain function-call forms!) are much nicer syntax.

However, there are some very good use cases for `.with()` and `.update()`.

For example, consider a signal that holds a `Vec<String>`.

```rust
let (names, set_names) = create_signal(cx, Vec::new());
if names().is_empty() {
	set_names(vec!["Alice".to_string()]);
}
```

In terms of logically, this is simple enough, but it’s hiding some significant inefficiencies. Remember that `names().is_empty()` is sugar for `names.get().is_empty()`, which clones the value (it’s `names.with(|n| n.clone()).is_empty()`). This means we clone the whole `Vec<String>`, run `is_empty()`, and then immediately throw away the clone.

Likewise, `set_names` replaces the value with a whole new `Vec<_>`. This is fine, but we might as well just mutate the original `Vec<_>` in place.

```rust
let (names, set_names) = create_signal(cx, Vec::new());
if names.with(|names| names.is_empty()) {
	set_names.update(|names| names.push("Alice".to_string()));
}
```

Now our function simply takes `names` by reference to run `is_empty()`, avoiding that clone.

And if you have Clippy on, or if you have sharp eyes, you may notice we can make this even neater:

```rust
if names.with(Vec::is_empty) {
	// ...
}
```

After all, `.with()` simply takes a function that takes the value by reference. Since `Vec::is_empty` takes `&self`, we can pass it in directly and avoid the unncessary closure.
