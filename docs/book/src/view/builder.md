# No Macros: The View Builder Syntax

> If you’re perfectly happy with the `view!` macro syntax described so far, you’re welcome to skip this chapter. The builder syntax described in this section is always available, but never required.

For one reason or another, many developers would prefer to avoid macros. Perhaps you don’t like the limited `rustfmt` support. (Although, you should check out [`leptosfmt`](https://github.com/bram209/leptosfmt), which is an excellent tool!) Perhaps you worry about the effect of macros on compile time. Perhaps you prefer the aesthetics of pure Rust syntax, or you have trouble context-switching between an HTML-like syntax and your Rust code. Or perhaps you want more flexibility in how you create and manipulate HTML elements than the `view` macro provides.

If you fall into any of those camps, the builder syntax may be for you.

The `view` macro expands an HTML-like syntax to a series of Rust functions and method calls. If you’d rather not use the `view` macro, you can simply use that expanded syntax yourself. And it’s actually pretty nice!

First off, if you want you can even drop the `#[component]` macro: a component is just a setup function that creates your view, so you can define a component as a simple function call:

```rust
pub fn counter(initial_value: i32, step: u32) -> impl IntoView { }
```

Elements are created by calling a function with the same name as the HTML element:

```rust
p()
```

You can add children to the element with [`.child()`](https://docs.rs/leptos/latest/leptos/struct.HtmlElement.html#method.child), which takes a single child or a tuple or array of types that implement [`IntoView`](https://docs.rs/leptos/latest/leptos/trait.IntoView.html).

```rust
p().child((em().child("Big, "), strong().child("bold "), "text"))
```

Attributes are added with [`.attr()`](https://docs.rs/leptos/latest/leptos/struct.HtmlElement.html#method.attr). This can take any of the same types that you could pass as an attribute into the view macro (types that implement [`IntoAttribute`](https://docs.rs/leptos/latest/leptos/trait.IntoAttribute.html)).

```rust
p().attr("id", "foo").attr("data-count", move || count().to_string())
```

Similarly, the `class:`, `prop:`, and `style:` syntaxes map directly onto [`.class()`](https://docs.rs/leptos/latest/leptos/struct.HtmlElement.html#method.class), [`.prop()`](https://docs.rs/leptos/latest/leptos/struct.HtmlElement.html#method.prop), and [`.style()`](https://docs.rs/leptos/latest/leptos/struct.HtmlElement.html#method.style) methods.

Event listeners can be added with [`.on()`](https://docs.rs/leptos/latest/leptos/struct.HtmlElement.html#method.on). Typed events found in [`leptos::ev`](https://docs.rs/leptos/latest/leptos/ev/index.html) prevent typos in event names and allow for correct type inference in the callback function.

```rust
button()
    .on(ev::click, move |_| set_count.update(|count| count.clear()))
    .child("Clear")
```

> Many additional methods can be found in the [`HtmlElement`](https://docs.rs/leptos/latest/leptos/struct.HtmlElement.html#method.child) docs, including some methods that are not directly available in the `view` macro.

All of this adds up to a very Rusty syntax to build full-featured views, if you prefer this style.

```rust
/// A simple counter view.
// A component is really just a function call: it runs once to create the DOM and reactive system
pub fn counter(initial_value: i32, step: u32) -> impl IntoView {
    let (count, set_count) = create_signal(0);

    div()
        .child((
            button()
                // typed events found in leptos::ev
                // 1) prevent typos in event names
                // 2) allow for correct type inference in callbacks
                .on(ev::click, move |_| set_count.update(|count| count.clear()))
                .child("Clear"),
            button()
                .on(ev::click, move |_| {
                    set_count.update(|count| count.decrease())
                })
                .child("-1"),
            span().child(("Value: ", move || count.get().value(), "!")),
            button()
                .on(ev::click, move |_| {
                    set_count.update(|count| count.increase())
                })
                .child("+1"),
        ))
}
```

This also has the benefit of being more flexible: because these are all plain Rust functions and methods, it’s easier to use them in things like iterator adapters without any additional “magic”:

```rust
// take some set of attribute names and values
let attrs: Vec<(&str, AttributeValue)> = todo!();
// you can use the builder syntax to “spread” these onto the
// element in a way that’s not possible with the view macro
let p = attrs
    .into_iter()
    .fold(p(), |el, (name, value)| el.attr(name, value));

```

> ## Performance Note
>
> One caveat: the `view` macro applies significant optimizations in server-side-rendering (SSR) mode to improve HTML rendering performance significantly (think 2-4x faster, depending on the characteristics of any given app). It does this by analyzing your `view` at compile time and converting the static parts into simple HTML strings, rather than expanding them into the builder syntax.
>
> This means two things:
>
> 1. The builder syntax and `view` macro should not be mixed, or should only be mixed very carefully: at least in SSR mode, the output of the `view` should be treated as a “black box” that can’t have additional builder methods applied to it without causing inconsistencies.
> 2. Using the builder syntax will result in less-than-optimal SSR performance. It won’t be slow, by any means (and it’s worth running your own benchmarks in any case), just slower than the `view`-optimized version.
