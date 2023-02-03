# Control Flow

In most applications, you sometimes need to make a decision: Should I render this
part of the view, or not? Should I render `<ButtonA/>` or `<WidgetB/>`? This is
**control flow**.

## A Few Tips

When thinking about how to do this with Leptos, it’s important to remember a few
things:

1. Rust is an expression-oriented language: control-flow expressions like
   `if x() { y } else { z }` and `match x() { ... }` return their values. This
   makes them very useful for declarative user interfaces.
2. For any `T` that implements `IntoView`—in other words, for any type that Leptos
   knows how to render—`Option<T>` and `Result<T, impl Error>` _also_ implement
   `IntoView`. And just as `Fn() -> T` renders a reactive `T`, `Fn() -> Option<T>`
   and `Fn() -> Result<T, impl Error>` are reactive.
3. Rust has lots of handy helpers like [Option::map](https://doc.rust-lang.org/std/option/enum.Option.html#method.map),
   [Option::and_then](https://doc.rust-lang.org/std/option/enum.Option.html#method.and_then),
   [Option::ok_or](https://doc.rust-lang.org/std/option/enum.Option.html#method.ok_or),
   [Result::map](https://doc.rust-lang.org/std/result/enum.Result.html#method.map),
   [Result::ok](https://doc.rust-lang.org/std/result/enum.Result.html#method.ok), and
   [bool::then](https://doc.rust-lang.org/std/primitive.bool.html#method.then) that
   allow you to convert, in a declarative way, between a few different standard types,
   all of which can be rendered. Spending time in the `Option` and `Result` docs in particular
   is one of the best ways to level up your Rust game.
4. And always remember: to be reactive, values must be functions. You’ll see me constantly
   wrap things in a `move ||` closure, below. This is to ensure that they actually re-run
   when the signal they depend on changes, keeping the UI reactive.

## So What?

To connect the dots a little: this means that you can actually implement most of
your control flow with native Rust code, without any control-flow components or
special knowledge.

For example, let’s start with a simple signal and derived signal:

```rust
let (value, set_value) = create_signal(cx, 0);
let is_odd = move || value() & 1 == 1;
```

> If you don’t recognize what’s going on with `is_odd`, don’t worry about it
> too much. It’s just a simple way to test whether an integer is odd by doing a
> bitwise `AND` with `1`.

We can use these signals and ordinary Rust to build most control flow.

### `if` statements

Let’s say I want to render some text if the number is odd, and some other text
if it’s even. Well, how about this?

```rust
view! { cx,
	<p>
	{move || if is_odd() {
		"Odd"
	} else {
		"Even"
	}}
	</p>
}
```

An `if` expression returns its value, and a `&str` implements `IntoView`, so a
`Fn() -> &str` implements `IntoView`, so this... just works!

### `Option<T>`

Let’s say we want to render some text if it’s odd, and nothing if it’s even.

```rust
let message = move || {
	if is_odd() {
		Some("Ding ding ding!")
	} else {
		None
	}
};

view! { cx,
	<p>{message}</p>
}
```

This works fine. We can make it a little shorter if we’d like, using `bool::then()`.

```rust
let message = move || is_odd().then(|| "Ding ding ding!");
view! { cx,
	<p>{message}</p>
}
```

You could even inline this if you’d like, although personally I sometimes like the
better `cargo fmt` and `rust-analyzer` support I get by pulling things out of the `view`.

### `match` statements

We’re still just writing ordinary Rust code, right? So you have all the power of Rust’s
pattern matching at your disposal.

```rust
let message = move || {
	match value() {
		0 => "Zero",
		1 => "One",
		n if is_odd() => "Odd",
		_ => "Even"
	}
};
view! { cx,
	<p>{message}</p>
}
```

And why not? YOLO, right?

## Preventing Over-Rendering

Not so YOLO.
