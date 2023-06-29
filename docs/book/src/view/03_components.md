# Components and Props

So far, we’ve been building our whole application in a single component. This
is fine for really tiny examples, but in any real application you’ll need to
break the user interface out into multiple components, so you can break your
interface down into smaller, reusable, composable chunks.

Let’s take our progress bar example. Imagine that you want two progress bars
instead of one: one that advances one tick per click, one that advances two ticks
per click.

You _could_ do this by just creating two `<progress>` elements:

```rust
let (count, set_count) = create_signal(cx, 0);
let double_count = move || count() * 2;

view! { cx,
    <progress
        max="50"
        value=count
    />
    <progress
        max="50"
        value=double_count
    />
}
```

But of course, this doesn’t scale very well. If you want to add a third progress
bar, you need to add this code another time. And if you want to edit anything
about it, you need to edit it in triplicate.

Instead, let’s create a `<ProgressBar/>` component.

```rust
#[component]
fn ProgressBar(
    cx: Scope
) -> impl IntoView {
    view! { cx,
        <progress
            max="50"
            // hmm... where will we get this from?
            value=progress
        />
    }
}
```

There’s just one problem: `progress` is not defined. Where should it come from?
When we were defining everything manually, we just used the local variable names.
Now we need some way to pass an argument into the component.

## Component Props

We do this using component properties, or “props.” If you’ve used another frontend
framework, this is probably a familiar idea. Basically, properties are to components
as attributes are to HTML elements: they let you pass additional information into
the component.

In Leptos, you define props by giving additional arguments to the component function.

```rust
#[component]
fn ProgressBar(
    cx: Scope,
    progress: ReadSignal<i32>
) -> impl IntoView {
    view! { cx,
        <progress
            max="50"
            // now this works
            value=progress
        />
    }
}
```

Now we can use our component in the main `<App/>` component’s view.

```rust
#[component]
fn App(cx: Scope) -> impl IntoView {
    let (count, set_count) = create_signal(cx, 0);
    view! { cx,
        <button on:click=move |_| { set_count.update(|n| *n += 1); }>
            "Click me"
        </button>
        // now we use our component!
        <ProgressBar progress=count/>
    }
}
```

Using a component in the view looks a lot like using an HTML element. You’ll
notice that you can easily tell the difference between an element and a component
because components always have `PascalCase` names. You pass the `progress` prop
in as if it were an HTML element attribute. Simple.

### Reactive and Static Props

You’ll notice that throughout this example, `progress` takes a reactive
`ReadSignal<i32>`, and not a plain `i32`. This is **very important**.

Component props have no special meaning attached to them. A component is simply
a function that runs once to set up the user interface. The only way to tell the
interface to respond to changing is to pass it a signal type. So if you have a
component property that will change over time, like our `progress`, it should
be a signal.

### `optional` Props

Right now the `max` setting is hard-coded. Let’s take that as a prop too. But
let’s add a catch: let’s make this prop optional by annotating the particular
argument to the component function with `#[prop(optional)]`.

```rust
#[component]
fn ProgressBar(
    cx: Scope,
    // mark this prop optional
    // you can specify it or not when you use <ProgressBar/>
    #[prop(optional)]
    max: u16,
    progress: ReadSignal<i32>
) -> impl IntoView {
    view! { cx,
        <progress
            max=max
            value=progress
        />
    }
}
```

Now, we can use `<ProgressBar max=50 value=count/>`, or we can omit `max`
to use the default value (i.e., `<ProgressBar value=count/>`). The default value
on an `optional` is its `Default::default()` value, which for a `u16` is going to
be `0`. In the case of a progress bar, a max value of `0` is not very useful.

So let’s give it a particular default value instead.

### `default` props

You can specify a default value other than `Default::default()` pretty simply
with `#[prop(default = ...)`.

```rust
#[component]
fn ProgressBar(
    cx: Scope,
    #[prop(default = 100)]
    max: u16,
    progress: ReadSignal<i32>
) -> impl IntoView {
    view! { cx,
        <progress
            max=max
            value=progress
        />
    }
}
```

### Generic Props

This is great. But we began with two counters, one driven by `count`, and one by
the derived signal `double_count`. Let’s recreate that by using `double_count`
as the `progress` prop on another `<ProgressBar/>`.

```rust
#[component]
fn App(cx: Scope) -> impl IntoView {
    let (count, set_count) = create_signal(cx, 0);
    let double_count = move || count() * 2;

    view! { cx,
        <button on:click=move |_| { set_count.update(|n| *n += 1); }>
            "Click me"
        </button>
        <ProgressBar progress=count/>
        // add a second progress bar
        <ProgressBar progress=double_count/>
    }
}
```

Hm... this won’t compile. It should be pretty easy to understand why: we’ve declared
that the `progress` prop takes `ReadSignal<i32>`, and `double_count` is not
`ReadSignal<i32>`. As rust-analyzer will tell you, its type is `|| -> i32`, i.e.,
it’s a closure that returns an `i32`.

There are a couple ways to handle this. One would be to say: “Well, I know that
a `ReadSignal` is a function, and I know that a closure is a function; maybe I
could just take any function?” If you’re savvy, you may know that both these
implement the trait `Fn() -> i32`. So you could use a generic component:

```rust
#[component]
fn ProgressBar<F>(
    cx: Scope,
    #[prop(default = 100)]
    max: u16,
    progress: F
) -> impl IntoView
where
    F: Fn() -> i32 + 'static,
{
    view! { cx,
        <progress
            max=max
            value=progress
        />
    }
}
```

This is a perfectly reasonable way to write this component: `progress` now takes
any value that implements this `Fn()` trait.

> Note that generic component props _cannot_ be specified inline (as `<F: Fn() -> i32>`)
> or as `progress: impl Fn() -> i32 + 'static,`, in part because they’re actually used to generate
> a `struct ProgressBarProps`, and struct fields cannot be `impl` types.

### `into` Props

There’s one more way we could implement this, and it would be to use `#[prop(into)]`.
This attribute automatically calls `.into()` on the values you pass as props,
which allows you to easily pass props with different values.

In this case, it’s helpful to know about the
[`Signal`](https://docs.rs/leptos/latest/leptos/struct.Signal.html) type. `Signal`
is an enumerated type that represents any kind of readable reactive signal. It can
be useful when defining APIs for components you’ll want to reuse while passing
different sorts of signals. The [`MaybeSignal`](https://docs.rs/leptos/latest/leptos/enum.MaybeSignal.html) type is useful when you want to be able to take either a static or
reactive value.

```rust
#[component]
fn ProgressBar(
    cx: Scope,
    #[prop(default = 100)]
    max: u16,
    #[prop(into)]
    progress: Signal<i32>
) -> impl IntoView
{
    view! { cx,
        <progress
            max=max
            value=progress
        />
    }
}

#[component]
fn App(cx: Scope) -> impl IntoView {
    let (count, set_count) = create_signal(cx, 0);
    let double_count = move || count() * 2;

    view! { cx,
        <button on:click=move |_| { set_count.update(|n| *n += 1); }>
            "Click me"
        </button>
        // .into() converts `ReadSignal` to `Signal`
        <ProgressBar progress=count/>
        // use `Signal::derive()` to wrap a derived signal
        <ProgressBar progress=Signal::derive(cx, double_count)/>
    }
}
```

## Documenting Components

This is one of the least essential but most important sections of this book.
It’s not strictly necessary to document your components and their props. It may
be very important, depending on the size of your team and your app. But it’s very
easy, and bears immediate fruit.

To document a component and its props, you can simply add doc comments on the
component function, and each one of the props:

```rust
/// Shows progress toward a goal.
#[component]
fn ProgressBar(
    cx: Scope,
    /// The maximum value of the progress bar.
    #[prop(default = 100)]
    max: u16,
    /// How much progress should be displayed.
    #[prop(into)]
    progress: Signal<i32>,
) -> impl IntoView {
    /* ... */
}
```

That’s all you need to do. These behave like ordinary Rust doc comments, except
that you can document individual component props, which can’t be done with Rust
function arguments.

This will automatically generate documentation for your component, its `Props`
type, and each of the fields used to add props. It can be a little hard to
understand how powerful this is until you hover over the component name or props
and see the power of the `#[component]` macro combined with rust-analyzer here.

[Click to open CodeSandbox.](https://codesandbox.io/p/sandbox/3-components-50t2e7?file=%2Fsrc%2Fmain.rs&selection=%5B%7B%22endColumn%22%3A1%2C%22endLineNumber%22%3A7%2C%22startColumn%22%3A1%2C%22startLineNumber%22%3A7%7D%5D)

<iframe src="https://codesandbox.io/p/sandbox/3-components-50t2e7?file=%2Fsrc%2Fmain.rs&selection=%5B%7B%22endColumn%22%3A1%2C%22endLineNumber%22%3A7%2C%22startColumn%22%3A1%2C%22startLineNumber%22%3A7%7D%5D" width="100%" height="1000px" style="max-height: 100vh"></iframe>
