# `view`: Dynamic Classes, Styles and Attributes

So far we’ve seen how to use the `view` macro to create event listeners and to
create dynamic text by passing a function (such as a signal) into the view.

But of course there are other things you might want to update in your user interface.
In this section, we’ll look at how to update classes, styles and attributes dynamically,
and we’ll introduce the concept of a **derived signal**.

Let’s start with a simple component that should be familiar: click a button to
increment a counter.

```rust
#[component]
fn App() -> impl IntoView {
    let (count, set_count) = create_signal(0);

    view! {
        <button
            on:click=move |_| {
                set_count.update(|n| *n += 1);
            }
        >
            "Click me: "
            {move || count()}
        </button>
    }
}
```

So far, this is just the example from the last chapter.

## Dynamic Classes

Now let’s say I’d like to update the list of CSS classes on this element dynamically.
For example, let’s say I want to add the class `red` when the count is odd. I can
do this using the `class:` syntax.

```rust
class:red=move || count() % 2 == 1
```

`class:` attributes take

1. the class name, following the colon (`red`)
2. a value, which can be a `bool` or a function that returns a `bool`

When the value is `true`, the class is added. When the value is `false`, the class
is removed. And if the value is a function that accesses a signal, the class will
reactively update when the signal changes.

Now every time I click the button, the text should toggle between red and black as
the number switches between even and odd.

Some CSS class names can’t be directly parsed by the `view` macro, especially if they include a mix of dashes and numbers or other characters. In that case, you can use a tuple syntax: `class=("name", value)` still directly updates a single class.

```rust
class=("button-20", move || count() % 2 == 1)
```

> If you’re following along, make sure you go into your `index.html` and add something like this:
>
> ```html
> <style>
>   .red {
>     color: red;
>   }
> </style>
> ```

## Dynamic Styles

Individual CSS properties can be directly updated with a similar `style:` syntax.

```rust
let (x, set_x) = create_signal(0);
let (y, set_y) = create_signal(0);
view! {
    <div
        style="position: absolute"
        style:left=move || format!("{}px", x() + 100)
        style:top=move || format!("{}px", y() + 100)
        style:background-color=move || format!("rgb({}, {}, 100)", x(), y())
        style=("--columns", x)
    >
        "Moves when coordinates change"
    </div>
}
```

## Dynamic Attributes

The same applies to plain attributes. Passing a plain string or primitive value to
an attribute gives it a static value. Passing a function (including a signal) to
an attribute causes it to update its value reactively. Let’s add another element
to our view:

```rust
<progress
    max="50"
    // signals are functions, so this <=> `move || count.get()`
    value=count
/>
```

Now every time we set the count, not only will the `class` of the `<button>` be
toggled, but the `value` of the `<progress>` bar will increase, which means that
our progress bar will move forward.

## Derived Signals

Let’s go one layer deeper, just for fun.

You already know that we create reactive interfaces just by passing functions into
the `view`. This means that we can easily change our progress bar. For example,
suppose we want it to move twice as fast:

```rust
<progress
    max="50"
    value=move || count() * 2
/>
```

But imagine we want to reuse that calculation in more than one place. You can do this
using a **derived signal**: a closure that accesses a signal.

```rust
let double_count = move || count() * 2;

/* insert the rest of the view */
<progress
    max="50"
    // we use it once here
    value=double_count
/>
<p>
    "Double Count: "
    // and again here
    {double_count}
</p>
```

Derived signals let you create reactive computed values that can be used in multiple
places in your application with minimal overhead.

Note: Using a derived signal like this means that the calculation runs once per
signal change (when `count()` changes) and once per place we access `double_count`;
in other words, twice. This is a very cheap calculation, so that’s fine.
We’ll look at memos in a later chapter, which re designed to solve this problem
for expensive calculations.

> #### Advanced Topic: Injecting Raw HTML
>
> The `view` macro provides support for an additional attribute, `inner_html`, which
> can be used to directly set the HTML contents of any element, wiping out any other
> children you’ve given it. Note that this does _not_ escape the HTML you provide. You
> should make sure that it only contains trusted input or that any HTML entities are
> escaped, to prevent cross-site scripting (XSS) attacks.
>
> ```rust
> let html = "<p>This HTML will be injected.</p>";
> view! {
>   <div inner_html=html/>
> }
> ```
>
> [Click here for the full `view` macros docs](https://docs.rs/leptos/latest/leptos/macro.view.html).

[Click to open CodeSandbox.](https://codesandbox.io/p/sandbox/2-dynamic-attributes-0-5-lwdrpm?file=%2Fsrc%2Fmain.rs%3A1%2C1)

<iframe src="https://codesandbox.io/p/sandbox/2-dynamic-attributes-0-5-lwdrpm?file=%2Fsrc%2Fmain.rs%3A1%2C1" width="100%" height="1000px" style="max-height: 100vh"></iframe>

<details>
<summary>Code Sandbox Source</summary>

```rust
use leptos::*;

#[component]
fn App() -> impl IntoView {
    let (count, set_count) = create_signal(0);

    // a "derived signal" is a function that accesses other signals
    // we can use this to create reactive values that depend on the
    // values of one or more other signals
    let double_count = move || count() * 2;

    view! {
        <button
            on:click=move |_| {
                set_count.update(|n| *n += 1);
            }
            // the class: syntax reactively updates a single class
            // here, we'll set the `red` class when `count` is odd
            class:red=move || count() % 2 == 1
        >
            "Click me"
        </button>
        // NOTE: self-closing tags like <br> need an explicit /
        <br/>

        // We'll update this progress bar every time `count` changes
        <progress
            // static attributes work as in HTML
            max="50"

            // passing a function to an attribute
            // reactively sets that attribute
            // signals are functions, so this <=> `move || count.get()`
            value=count
        >
        </progress>
        <br/>

        // This progress bar will use `double_count`
        // so it should move twice as fast!
        <progress
            max="50"
            // derived signals are functions, so they can also
            // reactive update the DOM
            value=double_count
        >
        </progress>
        <p>"Count: " {count}</p>
        <p>"Double Count: " {double_count}</p>
    }
}

fn main() {
    leptos::mount_to_body(App)
}

            // passing a function to an attribute
            // reactively sets that attribute
            // signals are functions, so this <=> `move || count.get()`
            value=count
        >
        </progress>
        <br/>

        // This progress bar will use `double_count`
        // so it should move twice as fast!
        <progress
            max="50"
            // derived signals are functions, so they can also
            // reactive update the DOM
            value=double_count
        >
        </progress>
        <p>"Count: " {count}</p>
        <p>"Double Count: " {double_count}</p>
    }
}

fn main() {
    leptos::mount_to_body(App)
}
```

</details>
</preview>
