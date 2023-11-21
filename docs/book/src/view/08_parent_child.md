# Parent-Child Communication

You can think of your application as a nested tree of components. Each component
handles its own local state and manages a section of the user interface, so
components tend to be relatively self-contained.

Sometimes, though, you’ll want to communicate between a parent component and its
child. For example, imagine you’ve defined a `<FancyButton/>` component that adds
some styling, logging, or something else to a `<button/>`. You want to use a
`<FancyButton/>` in your `<App/>` component. But how can you communicate between
the two?

It’s easy to communicate state from a parent component to a child component. We
covered some of this in the material on [components and props](./03_components.md).
Basically if you want the parent to communicate to the child, you can pass a
[`ReadSignal`](https://docs.rs/leptos/latest/leptos/struct.ReadSignal.html), a
[`Signal`](https://docs.rs/leptos/latest/leptos/struct.Signal.html), or even a
[`MaybeSignal`](https://docs.rs/leptos/latest/leptos/enum.MaybeSignal.html) as a prop.

But what about the other direction? How can a child send notifications about events
or state changes back up to the parent?

There are four basic patterns of parent-child communication in Leptos.

## 1. Pass a [`WriteSignal`](https://docs.rs/leptos/latest/leptos/struct.WriteSignal.html)

One approach is simply to pass a `WriteSignal` from the parent down to the child, and update
it in the child. This lets you manipulate the state of the parent from the child.

```rust
#[component]
pub fn App() -> impl IntoView {
    let (toggled, set_toggled) = create_signal(false);
    view! {
        <p>"Toggled? " {toggled}</p>
        <ButtonA setter=set_toggled/>
    }
}

#[component]
pub fn ButtonA(setter: WriteSignal<bool>) -> impl IntoView {
    view! {
        <button
            on:click=move |_| setter.update(|value| *value = !*value)
        >
            "Toggle"
        </button>
    }
}
```

This pattern is simple, but you should be careful with it: passing around a `WriteSignal`
can make it hard to reason about your code. In this example, it’s pretty clear when you
read `<App/>` that you are handing off the ability to mutate `toggled`, but it’s not at
all clear when or how it will change. In this small, local example it’s easy to understand,
but if you find yourself passing around `WriteSignal`s like this throughout your code,
you should really consider whether this is making it too easy to write spaghetti code.

## 2. Use a Callback

Another approach would be to pass a callback to the child: say, `on_click`.

```rust
#[component]
pub fn App() -> impl IntoView {
    let (toggled, set_toggled) = create_signal(false);
    view! {
        <p>"Toggled? " {toggled}</p>
        <ButtonB on_click=move |_| set_toggled.update(|value| *value = !*value)/>
    }
}


#[component]
pub fn ButtonB(#[prop(into)] on_click: Callback<MouseEvent>) -> impl IntoView
{
    view! {
        <button on:click=on_click>
            "Toggle"
        </button>
    }
}
```

You’ll notice that whereas `<ButtonA/>` was given a `WriteSignal` and decided how to mutate it,
`<ButtonB/>` simply fires an event: the mutation happens back in `<App/>`. This has the advantage
of keeping local state local, preventing the problem of spaghetti mutation. But it also means
the logic to mutate that signal needs to exist up in `<App/>`, not down in `<ButtonB/>`. These
are real trade-offs, not a simple right-or-wrong choice.

> Note the way we use the `Callback<In, Out>` type. This is basically a
> wrapper around a closure `Fn(In) -> Out` that is also `Copy` and makes it
> easy to pass around.
> 
> We also used the `#[prop(into)]` attribute so we can pass a normal closure into
> `on_click`. Please see the [chapter "`into` Props"](./03_components.md#into-props) for more details.

### 2.1 Use Closure instead of `Callback`

You can use a Rust closure `Fn(MouseEvent)` directly instead of `Callback`:

```rust
#[component]
pub fn App() -> impl IntoView {
    let (toggled, set_toggled) = create_signal(false);
    view! {
        <p>"Toggled? " {toggled}</p>
        <ButtonB on_click=move |_| set_toggled.update(|value| *value = !*value)/>
    }
}


#[component]
pub fn ButtonB<F>(on_click: F) -> impl IntoView
where
    F: Fn(MouseEvent) + 'static
{
    view! {
        <button on:click=on_click>
            "Toggle"
        </button>
    }
}
```

The code is very similar in this case. On more advanced use-cases using a
closure might require some cloning compared to using a `Callback`.

> Note the way we declare the generic type `F` here for the callback. If you’re
> confused, look back at the [generic props](./03_components.html#generic-props) section
> of the chapter on components.


## 3. Use an Event Listener

You can actually write Option 2 in a slightly different way. If the callback maps directly onto
a native DOM event, you can add an `on:` listener directly to the place you use the component
in your `view` macro in `<App/>`.

```rust
#[component]
pub fn App() -> impl IntoView {
    let (toggled, set_toggled) = create_signal(false);
    view! {
        <p>"Toggled? " {toggled}</p>
        // note the on:click instead of on_click
        // this is the same syntax as an HTML element event listener
        <ButtonC on:click=move |_| set_toggled.update(|value| *value = !*value)/>
    }
}


#[component]
pub fn ButtonC() -> impl IntoView {
    view! {
        <button>"Toggle"</button>
    }
}
```

This lets you write way less code in `<ButtonC/>` than you did for `<ButtonB/>`,
and still gives a correctly-typed event to the listener. This works by adding an
`on:` event listener to each element that `<ButtonC/>` returns: in this case, just
the one `<button>`.

Of course, this only works for actual DOM events that you’re passing directly through
to the elements you’re rendering in the component. For more complex logic that
doesn’t map directly onto an element (say you create `<ValidatedForm/>` and want an
`on_valid_form_submit` callback) you should use Option 2.

## 4. Providing a Context

This version is actually a variant on Option 1. Say you have a deeply-nested component
tree:

```rust
#[component]
pub fn App() -> impl IntoView {
    let (toggled, set_toggled) = create_signal(false);
    view! {
        <p>"Toggled? " {toggled}</p>
        <Layout/>
    }
}

#[component]
pub fn Layout() -> impl IntoView {
    view! {
        <header>
            <h1>"My Page"</h1>
        </header>
        <main>
            <Content/>
        </main>
    }
}

#[component]
pub fn Content() -> impl IntoView {
    view! {
        <div class="content">
            <ButtonD/>
        </div>
    }
}

#[component]
pub fn ButtonD<F>() -> impl IntoView {
    todo!()
}
```

Now `<ButtonD/>` is no longer a direct child of `<App/>`, so you can’t simply
pass your `WriteSignal` to its props. You could do what’s sometimes called
“prop drilling,” adding a prop to each layer between the two:

```rust
#[component]
pub fn App() -> impl IntoView {
    let (toggled, set_toggled) = create_signal(false);
    view! {
        <p>"Toggled? " {toggled}</p>
        <Layout set_toggled/>
    }
}

#[component]
pub fn Layout(set_toggled: WriteSignal<bool>) -> impl IntoView {
    view! {
        <header>
            <h1>"My Page"</h1>
        </header>
        <main>
            <Content set_toggled/>
        </main>
    }
}

#[component]
pub fn Content(set_toggled: WriteSignal<bool>) -> impl IntoView {
    view! {
        <div class="content">
            <ButtonD set_toggled/>
        </div>
    }
}

#[component]
pub fn ButtonD<F>(set_toggled: WriteSignal<bool>) -> impl IntoView {
    todo!()
}
```

This is a mess. `<Layout/>` and `<Content/>` don’t need `set_toggled`; they just
pass it through to `<ButtonD/>`. But I need to declare the prop in triplicate.
This is not only annoying but hard to maintain: imagine we add a “half-toggled”
option and the type of `set_toggled` needs to change to an `enum`. We have to change
it in three places!

Isn’t there some way to skip levels?

There is!

### 4.1 The Context API

You can provide data that skips levels by using [`provide_context`](https://docs.rs/leptos/latest/leptos/fn.provide_context.html)
and [`use_context`](https://docs.rs/leptos/latest/leptos/fn.use_context.html). Contexts are identified
by the type of the data you provide (in this example, `WriteSignal<bool>`), and they exist in a top-down
tree that follows the contours of your UI tree. In this example, we can use context to skip the
unnecessary prop drilling.

```rust
#[component]
pub fn App() -> impl IntoView {
    let (toggled, set_toggled) = create_signal(false);

    // share `set_toggled` with all children of this component
    provide_context(set_toggled);

    view! {
        <p>"Toggled? " {toggled}</p>
        <Layout/>
    }
}

// <Layout/> and <Content/> omitted
// To work in this version, drop their references to set_toggled

#[component]
pub fn ButtonD() -> impl IntoView {
    // use_context searches up the context tree, hoping to
    // find a `WriteSignal<bool>`
    // in this case, I .expect() because I know I provided it
    let setter = use_context::<WriteSignal<bool>>()
        .expect("to have found the setter provided");

    view! {
        <button
            on:click=move |_| setter.update(|value| *value = !*value)
        >
            "Toggle"
        </button>
    }
}
```

The same caveats apply to this as to `<ButtonA/>`: passing a `WriteSignal`
around should be done with caution, as it allows you to mutate state from
arbitrary parts of your code. But when done carefully, this can be one of
the most effective techniques for global state management in Leptos: simply
provide the state at the highest level you’ll need it, and use it wherever
you need it lower down.

Note that there are no performance downsides to this approach. Because you
are passing a fine-grained reactive signal, _nothing happens_ in the intervening
components (`<Layout/>` and `<Content/>`) when you update it. You are communicating
directly between `<ButtonD/>` and `<App/>`. In fact—and this is the power of
fine-grained reactivity—you are communicating directly between a button click
in `<ButtonD/>` and a single text node in `<App/>`. It’s as if the components
themselves don’t exist at all. And, well... at runtime, they don’t. It’s just
signals and effects, all the way down.

[Click to open CodeSandbox.](https://codesandbox.io/p/sandbox/8-parent-child-0-5-7rz7qd?file=%2Fsrc%2Fmain.rs%3A1%2C2)

<iframe src="https://codesandbox.io/p/sandbox/8-parent-child-0-5-7rz7qd?file=%2Fsrc%2Fmain.rs%3A1%2C2" width="100%" height="1000px" style="max-height: 100vh"></iframe>

<details>
<summary>CodeSandbox Source</summary>

```rust
use leptos::{ev::MouseEvent, *};

// This highlights four different ways that child components can communicate
// with their parent:
// 1) <ButtonA/>: passing a WriteSignal as one of the child component props,
//    for the child component to write into and the parent to read
// 2) <ButtonB/>: passing a closure as one of the child component props, for
//    the child component to call
// 3) <ButtonC/>: adding an `on:` event listener to a component
// 4) <ButtonD/>: providing a context that is used in the component (rather than prop drilling)

#[derive(Copy, Clone)]
struct SmallcapsContext(WriteSignal<bool>);

#[component]
pub fn App() -> impl IntoView {
    // just some signals to toggle three classes on our <p>
    let (red, set_red) = create_signal(false);
    let (right, set_right) = create_signal(false);
    let (italics, set_italics) = create_signal(false);
    let (smallcaps, set_smallcaps) = create_signal(false);

    // the newtype pattern isn't *necessary* here but is a good practice
    // it avoids confusion with other possible future `WriteSignal<bool>` contexts
    // and makes it easier to refer to it in ButtonC
    provide_context(SmallcapsContext(set_smallcaps));

    view! {
        <main>
            <p
                // class: attributes take F: Fn() => bool, and these signals all implement Fn()
                class:red=red
                class:right=right
                class:italics=italics
                class:smallcaps=smallcaps
            >
                "Lorem ipsum sit dolor amet."
            </p>

            // Button A: pass the signal setter
            <ButtonA setter=set_red/>

            // Button B: pass a closure
            <ButtonB on_click=move |_| set_right.update(|value| *value = !*value)/>

            // Button B: use a regular event listener
            // setting an event listener on a component like this applies it
            // to each of the top-level elements the component returns
            <ButtonC on:click=move |_| set_italics.update(|value| *value = !*value)/>

            // Button D gets its setter from context rather than props
            <ButtonD/>
        </main>
    }
}

/// Button A receives a signal setter and updates the signal itself
#[component]
pub fn ButtonA(
    /// Signal that will be toggled when the button is clicked.
    setter: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <button
            on:click=move |_| setter.update(|value| *value = !*value)
        >
            "Toggle Red"
        </button>
    }
}

/// Button B receives a closure
#[component]
pub fn ButtonB<F>(
    /// Callback that will be invoked when the button is clicked.
    on_click: F,
) -> impl IntoView
where
    F: Fn(MouseEvent) + 'static,
{
    view! {
        <button
            on:click=on_click
        >
            "Toggle Right"
        </button>
    }

    // just a note: in an ordinary function ButtonB could take on_click: impl Fn(MouseEvent) + 'static
    // and save you from typing out the generic
    // the component macro actually expands to define a
    //
    // struct ButtonBProps<F> where F: Fn(MouseEvent) + 'static {
    //   on_click: F
    // }
    //
    // this is what allows us to have named props in our component invocation,
    // instead of an ordered list of function arguments
    // if Rust ever had named function arguments we could drop this requirement
}

/// Button C is a dummy: it renders a button but doesn't handle
/// its click. Instead, the parent component adds an event listener.
#[component]
pub fn ButtonC() -> impl IntoView {
    view! {
        <button>
            "Toggle Italics"
        </button>
    }
}

/// Button D is very similar to Button A, but instead of passing the setter as a prop
/// we get it from the context
#[component]
pub fn ButtonD() -> impl IntoView {
    let setter = use_context::<SmallcapsContext>().unwrap().0;

    view! {
        <button
            on:click=move |_| setter.update(|value| *value = !*value)
        >
            "Toggle Small Caps"
        </button>
    }
}

fn main() {
    leptos::mount_to_body(App)
}
```

</details>
</preview>
