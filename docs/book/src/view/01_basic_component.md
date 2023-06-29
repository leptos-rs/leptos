# A Basic Component

That “Hello, world!” was a _very_ simple example. Let’s move on to something a
little more like an ordinary app.

First, let’s edit the `main` function so that, instead of rendering the whole
app, it just renders an `<App/>` component. Components are the basic unit of
composition and design in most web frameworks, and Leptos is no exception.
Conceptually, they are similar to HTML elements: they represent a section of the
DOM, with self-contained, defined behavior. Unlike HTML elements, they are in
`PascalCase`, so most Leptos applications will start with something like an
`<App/>` component.

```rust
fn main() {
    leptos::mount_to_body(|cx| view! { cx, <App/> })
}
```

Now let’s define our `<App/>` component itself. Because it’s relatively simple,
I’ll give you the whole thing up front, then walk through it line by line.

```rust
#[component]
fn App(cx: Scope) -> impl IntoView {
    let (count, set_count) = create_signal(cx, 0);

    view! { cx,
        <button
            on:click=move |_| {
                set_count(3);
            }
        >
            "Click me: "
            {move || count.get()}
        </button>
    }
}
```

## The Component Signature

```rust
#[component]
```

Like all component definitions, this begins with the [`#[component]`](https://docs.rs/leptos/latest/leptos/attr.component.html) macro. `#[component]` annotates a function so it can be
used as a component in your Leptos application. We’ll see some of the other features of
this macro in a couple chapters.

```rust
fn App(cx: Scope) -> impl IntoView
```

Every component is a function with the following characteristics

1. It takes a reactive [`Scope`](https://docs.rs/leptos/latest/leptos/struct.Scope.html)
   as its first argument. This `Scope` is our entrypoint into the reactive system.
   By convention, it’s usually named `cx`.
2. You can include other arguments, which will be available as component “props.”
3. Component functions return `impl IntoView`, which is an opaque type that includes
   anything you could return from a Leptos `view`.

## The Component Body

The body of the component function is a set-up function that runs once, not a
render function that reruns multiple times. You’ll typically use it to create a
few reactive variables, define any side effects that run in response to those values
changing, and describe the user interface.

```rust
let (count, set_count) = create_signal(cx, 0);
```

[`create_signal`](https://docs.rs/leptos/latest/leptos/fn.create_signal.html)
creates a signal, the basic unit of reactive change and state management in Leptos.
This returns a `(getter, setter)` tuple. To access the current value, you’ll
use `count.get()` (or, on `nightly` Rust, the shorthand `count()`). To set the
current value, you’ll call `set_count.set(...)` (or `set_count(...)`).

> `.get()` clones the value and `.set()` overwrites it. In many cases, it’s more efficient to use `.with()` or `.update()`; check out the docs for [`ReadSignal`](https://docs.rs/leptos/latest/leptos/struct.ReadSignal.html) and [`WriteSignal`](https://docs.rs/leptos/latest/leptos/struct.WriteSignal.html) if you’d like to learn more about those trade-offs at this point.

## The View

Leptos defines user interfaces using a JSX-like format via the [`view`](https://docs.rs/leptos/latest/leptos/macro.view.html) macro.

```rust
view! { cx,
    <button
        // define an event listener with on:
        on:click=move |_| {
            // on stable, this is set_count.set(3);
            set_count(3);
        }
    >
        // text nodes are wrapped in quotation marks
        "Click me: "
        // blocks can include Rust code
        {move || count.get()}
    </button>
}
```

This should mostly be easy to understand: it looks like HTML, with a special
`on:click` to define a `click` event listener, a text node that’s formatted like
a Rust string, and then...

```rust
{move || count.get()}
```

whatever that is.

People sometimes joke that they use more closures in their first Leptos application
than they’ve ever used in their lives. And fair enough. Basically, passing a function
into the view tells the framework: “Hey, this is something that might change.”

When we click the button and call `set_count`, the `count` signal is updated. This
`move || count.get()` closure, whose value depends on the value of `count`, reruns,
and the framework makes a targeted update to that one specific text node, touching
nothing else in your application. This is what allows for extremely efficient updates
to the DOM.

Now, if you have Clippy on—or if you have a particularly sharp eye—you might notice
that this closure is redundant, at least if you’re in `nightly` Rust. If you’re using
Leptos with `nightly` Rust, signals are already functions, so the closure is unnecessary.
As a result, you can write a simpler view:

```rust
view! { cx,
    <button /* ... */>
        "Click me: "
        // identical to {move || count.get()}
        {count}
    </button>
}
```

Remember—and this is _very important_—only functions are reactive. This means that
`{count}` and `{count()}` do very different things in your view. `{count}` passes
in a function, telling the framework to update the view every time `count` changes.
`{count()}` access the value of `count` once, and passes an `i32` into the view,
rendering it once, unreactively. You can see the difference in the CodeSandbox below!

Let’s make one final change. `set_count(3)` is a pretty useless thing for a click handler to do. Let’s replace “set this value to 3” with “increment this value by 1”:

```rust
move |_| {
    set_count.update(|n| *n += 1);
}
```

You can see here that while `set_count` just sets the value, `set_count.update()` gives us a mutable reference and mutates the value in place. Either one will trigger a reactive update in our UI.

> Throughout this tutorial, we’ll use CodeSandbox to show interactive examples. To
> show the browser in the sandbox, you may need to click `Add DevTools >
Other Previews > 8080.` Hover over any of the variables to show Rust-Analyzer details
> and docs for what’s going on. Feel free to fork the examples to play with them yourself!

[Click to open CodeSandbox.](https://codesandbox.io/p/sandbox/1-basic-component-3d74p3?file=%2Fsrc%2Fmain.rs&selection=%5B%7B%22endColumn%22%3A31%2C%22endLineNumber%22%3A19%2C%22startColumn%22%3A31%2C%22startLineNumber%22%3A19%7D%5D)

<iframe src="https://codesandbox.io/p/sandbox/1-basic-component-3d74p3?file=%2Fsrc%2Fmain.rs&selection=%5B%7B%22endColumn%22%3A31%2C%22endLineNumber%22%3A19%2C%22startColumn%22%3A31%2C%22startLineNumber%22%3A19%7D%5D" width="100%" height="1000px" style="max-height: 100vh"></iframe>
