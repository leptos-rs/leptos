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
    leptos::mount_to_body(|| view! { <App/> })
}
```

Now let’s define our `<App/>` component itself. Because it’s relatively simple,
I’ll give you the whole thing up front, then walk through it line by line.

```rust
#[component]
fn App() -> impl IntoView {
    let (count, set_count) = create_signal(0);

    view! {
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
fn App() -> impl IntoView
```

Every component is a function with the following characteristics

1. It takes zero or more arguments of any type.
2. It returns `impl IntoView`, which is an opaque type that includes
   anything you could return from a Leptos `view`.

> Component function arguments are gathered together into a single props struct which is built by the `view` macro as needed.

## The Component Body

The body of the component function is a set-up function that runs once, not a
render function that reruns multiple times. You’ll typically use it to create a
few reactive variables, define any side effects that run in response to those values
changing, and describe the user interface.

```rust
let (count, set_count) = create_signal(0);
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
view! {
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
view! {
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
`{count()}` accesses the value of `count` once, and passes an `i32` into the view,
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

[Click to open CodeSandbox.](https://codesandbox.io/p/sandbox/1-basic-component-3d74p3?file=%2Fsrc%2Fmain.rs%3A1%2C1)

<iframe src="https://codesandbox.io/p/sandbox/1-basic-component-3d74p3?file=%2Fsrc%2Fmain.rs%3A1%2C1" width="100%" height="1000px" style="max-height: 100vh"></iframe>

<details>
<summary>CodeSandbox Source</summary>

```rust
use leptos::*;

// The #[component] macro marks a function as a reusable component
// Components are the building blocks of your user interface
// They define a reusable unit of behavior
#[component]
fn App() -> impl IntoView {
    // here we create a reactive signal
    // and get a (getter, setter) pair
    // signals are the basic unit of change in the framework
    // we'll talk more about them later
    let (count, set_count) = create_signal(0);

    // the `view` macro is how we define the user interface
    // it uses an HTML-like format that can accept certain Rust values
    view! {
        <button
            // on:click will run whenever the `click` event fires
            // every event handler is defined as `on:{eventname}`

            // we're able to move `set_count` into the closure
            // because signals are Copy and 'static
            on:click=move |_| {
                set_count.update(|n| *n += 1);
            }
        >
            // text nodes in RSX should be wrapped in quotes,
            // like a normal Rust string
            "Click me"
        </button>
        <p>
            <strong>"Reactive: "</strong>
            // you can insert Rust expressions as values in the DOM
            // by wrapping them in curly braces
            // if you pass in a function, it will reactively update
            {move || count.get()}
        </p>
        <p>
            <strong>"Reactive shorthand: "</strong>
            // signals are functions, so we can remove the wrapping closure
            {count}
        </p>
        <p>
            <strong>"Not reactive: "</strong>
            // NOTE: if you write {count()}, this will *not* be reactive
            // it simply gets the value of count once
            {count()}
        </p>
    }
}

// This `main` function is the entry point into the app
// It just mounts our component to the <body>
// Because we defined it as `fn App`, we can now use it in a
// template as <App/>
fn main() {
    leptos::mount_to_body(|| view! { <App/> })
}
```
