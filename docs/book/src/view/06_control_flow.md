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
   wrap things in a `move ||` closure, below. This is to ensure that they actually rerun
   when the signal they depend on changes, keeping the UI reactive.

## So What?

To connect the dots a little: this means that you can actually implement most of
your control flow with native Rust code, without any control-flow components or
special knowledge.

For example, let’s start with a simple signal and derived signal:

```rust
let (value, set_value) = create_signal(0);
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
view! {
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

view! {
    <p>{message}</p>
}
```

This works fine. We can make it a little shorter if we’d like, using `bool::then()`.

```rust
let message = move || is_odd().then(|| "Ding ding ding!");
view! {
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
view! {
    <p>{message}</p>
}
```

And why not? YOLO, right?

## Preventing Over-Rendering

Not so YOLO.

Everything we’ve just done is basically fine. But there’s one thing you should remember
and try to be careful with. Each one of the control-flow functions we’ve created so far
is basically a derived signal: it will rerun every time the value changes. In the examples
above, where the value switches from even to odd on every change, this is fine.

But consider the following example:

```rust
let (value, set_value) = create_signal(0);

let message = move || if value() > 5 {
    "Big"
} else {
    "Small"
};

view! {
    <p>{message}</p>
}
```

This _works_, for sure. But if you added a log, you might be surprised

```rust
let message = move || if value() > 5 {
    logging::log!("{}: rendering Big", value());
    "Big"
} else {
    logging::log!("{}: rendering Small", value());
    "Small"
};
```

As a user clicks a button, you’d see something like this:

```
1: rendering Small
2: rendering Small
3: rendering Small
4: rendering Small
5: rendering Small
6: rendering Big
7: rendering Big
8: rendering Big
... ad infinitum
```

Every time `value` changes, it reruns the `if` statement. This makes sense, with
how reactivity works. But it has a downside. For a simple text node, rerunning
the `if` statement and rerendering isn’t a big deal. But imagine it were
like this:

```rust
let message = move || if value() > 5 {
    <Big/>
} else {
    <Small/>
};
```

This rerenders `<Small/>` five times, then `<Big/>` infinitely. If they’re
loading resources, creating signals, or even just creating DOM nodes, this is
unnecessary work.

### `<Show/>`

The [`<Show/>`](https://docs.rs/leptos/latest/leptos/fn.Show.html) component is
the answer. You pass it a `when` condition function, a `fallback` to be shown if
the `when` function returns `false`, and children to be rendered if `when` is `true`.

```rust
let (value, set_value) = create_signal(0);

view! {
  <Show
    when=move || { value() > 5 }
    fallback=|| view! { <Small/> }
  >
    <Big/>
  </Show>
}
```

`<Show/>` memoizes the `when` condition, so it only renders its `<Small/>` once,
continuing to show the same component until `value` is greater than five;
then it renders `<Big/>` once, continuing to show it indefinitely or until `value`
goes below five and then renders `<Small/>` again.

This is a helpful tool to avoid rerendering when using dynamic `if` expressions.
As always, there's some overhead: for a very simple node (like updating a single
text node, or updating a class or attribute), a `move || if ...` will be more
efficient. But if it’s at all expensive to render either branch, reach for
`<Show/>`.

## Note: Type Conversions

There‘s one final thing it’s important to say in this section.

The `view` macro doesn’t return the most-generic wrapping type
[`View`](https://docs.rs/leptos/latest/leptos/enum.View.html).
Instead, it returns things with types like `Fragment` or `HtmlElement<Input>`. This
can be a little annoying if you’re returning different HTML elements from
different branches of a conditional:

```rust,compile_error
view! {
    <main>
        {move || match is_odd() {
            true if value() == 1 => {
                // returns HtmlElement<Pre>
                view! { <pre>"One"</pre> }
            },
            false if value() == 2 => {
                // returns HtmlElement<P>
                view! { <p>"Two"</p> }
            }
            // returns HtmlElement<Textarea>
            _ => view! { <textarea>{value()}</textarea> }
        }}
    </main>
}
```

This strong typing is actually very powerful, because
[`HtmlElement`](https://docs.rs/leptos/0.1.3/leptos/struct.HtmlElement.html) is,
among other things, a smart pointer: each `HtmlElement<T>` type implements
`Deref` for the appropriate underlying `web_sys` type. In other words, in the browser
your `view` returns real DOM elements, and you can access native DOM methods on
them.

But it can be a little annoying in conditional logic like this, because you can’t
return different types from different branches of a condition in Rust. There are two ways
to get yourself out of this situation:

1. If you have multiple `HtmlElement` types, convert them to `HtmlElement<AnyElement>`
   with [`.into_any()`](https://docs.rs/leptos/latest/leptos/struct.HtmlElement.html#method.into_any)
2. If you have a variety of view types that are not all `HtmlElement`, convert them to
   `View`s with [`.into_view()`](https://docs.rs/leptos/latest/leptos/trait.IntoView.html#tymethod.into_view).

Here’s the same example, with the conversion added:

```rust,compile_error
view! {
    <main>
        {move || match is_odd() {
            true if value() == 1 => {
                // returns HtmlElement<Pre>
                view! { <pre>"One"</pre> }.into_any()
            },
            false if value() == 2 => {
                // returns HtmlElement<P>
                view! { <p>"Two"</p> }.into_any()
            }
            // returns HtmlElement<Textarea>
            _ => view! { <textarea>{value()}</textarea> }.into_any()
        }}
    </main>
}
```

[Click to open CodeSandbox.](https://codesandbox.io/p/sandbox/6-control-flow-0-5-4yn7qz?file=%2Fsrc%2Fmain.rs%3A1%2C1)

<iframe src="https://codesandbox.io/p/sandbox/6-control-flow-0-5-4yn7qz?file=%2Fsrc%2Fmain.rs%3A1%2C1" width="100%" height="1000px" style="max-height: 100vh"></iframe>

<details>
<summary>CodeSandbox Source</summary>

```rust
use leptos::*;

#[component]
fn App() -> impl IntoView {
    let (value, set_value) = create_signal(0);
    let is_odd = move || value() & 1 == 1;
    let odd_text = move || if is_odd() { Some("How odd!") } else { None };

    view! {
        <h1>"Control Flow"</h1>

        // Simple UI to update and show a value
        <button on:click=move |_| set_value.update(|n| *n += 1)>
            "+1"
        </button>
        <p>"Value is: " {value}</p>

        <hr/>

        <h2><code>"Option<T>"</code></h2>
        // For any `T` that implements `IntoView`,
        // so does `Option<T>`

        <p>{odd_text}</p>
        // This means you can use `Option` methods on it
        <p>{move || odd_text().map(|text| text.len())}</p>

        <h2>"Conditional Logic"</h2>
        // You can do dynamic conditional if-then-else
        // logic in several ways
        //
        // a. An "if" expression in a function
        //    This will simply re-render every time the value
        //    changes, which makes it good for lightweight UI
        <p>
            {move || if is_odd() {
                "Odd"
            } else {
                "Even"
            }}
        </p>

        // b. Toggling some kind of class
        //    This is smart for an element that's going to
        //    toggled often, because it doesn't destroy
        //    it in between states
        //    (you can find the `hidden` class in `index.html`)
        <p class:hidden=is_odd>"Appears if even."</p>

        // c. The <Show/> component
        //    This only renders the fallback and the child
        //    once, lazily, and toggles between them when
        //    needed. This makes it more efficient in many cases
        //    than a {move || if ...} block
        <Show when=is_odd
            fallback=|| view! { <p>"Even steven"</p> }
        >
            <p>"Oddment"</p>
        </Show>

        // d. Because `bool::then()` converts a `bool` to
        //    `Option`, you can use it to create a show/hide toggled
        {move || is_odd().then(|| view! { <p>"Oddity!"</p> })}

        <h2>"Converting between Types"</h2>
        // e. Note: if branches return different types,
        //    you can convert between them with
        //    `.into_any()` (for different HTML element types)
        //    or `.into_view()` (for all view types)
        {move || match is_odd() {
            true if value() == 1 => {
                // <pre> returns HtmlElement<Pre>
                view! { <pre>"One"</pre> }.into_any()
            },
            false if value() == 2 => {
                // <p> returns HtmlElement<P>
                // so we convert into a more generic type
                view! { <p>"Two"</p> }.into_any()
            }
            _ => view! { <textarea>{value()}</textarea> }.into_any()
        }}
    }
}

fn main() {
    leptos::mount_to_body(App)
}
```

</details>
</preview>
