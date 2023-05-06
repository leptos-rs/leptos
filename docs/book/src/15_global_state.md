# Global State Management

So far, we've only been working with local state in components
We've only seen how to communicate between parent and child components
But there are also more general ways to manage global state

The three best approaches to global state are

1. Using the router to drive global state via the URL
2. Passing signals through context
3. Creating a global state struct and creating lenses into it with `create_slice`

## Option #1: URL as Global State

The next few sections of the tutorial will be about the router.
So for now, we'll just look at options #2 and #3.

## Option #2: Passing Signals through Context

In virtual DOM libraries like React, using the Context API to manage global
state is a bad idea: because the entire app exists in a tree, changing
some value provided high up in the tree can cause the whole app to render.

In fine-grained reactive libraries like Leptos, this is simply not the case.
You can create a signal in the root of your app and pass it down to other
components using provide_context(). Changing it will only cause rerendering
in the specific places it is actually used, not the whole app.

We start by creating a signal in the root of the app and providing it to
all its children and descendants using `provide_context`.

```rust
#[component]
fn App(cx: Scope) -> impl IntoView {
    // here we create a signal in the root that can be consumed
    // anywhere in the app.
    let (count, set_count) = create_signal(cx, 0);
    // we'll pass the setter to specific components,
    // but provide the count itself to the whole app via context
    provide_context(cx, count);

    view! { cx,
        // SetterButton is allowed to modify the count
        <SetterButton set_count/>
        // These consumers can only read from it
        // But we could give them write access by passing `set_count` if we wanted
        <FancyMath/>
        <ListItems/>
    }
}
```

`<SetterButton/>` is the kind of counter we’ve written several times now.
(See the sandbox below if you don’t understand what I mean.)

`<FancyMath/>` and `<ListItems/>` both consume the signal we’re providing via
`use_context` and do something with it.

```rust
/// A component that does some "fancy" math with the global count
#[component]
fn FancyMath(cx: Scope) -> impl IntoView {
    // here we consume the global count signal with `use_context`
    let count = use_context::<ReadSignal<u32>>(cx)
        // we know we just provided this in the parent component
        .expect("there to be a `count` signal provided");
    let is_even = move || count() & 1 == 0;

    view! { cx,
        <div class="consumer blue">
            "The number "
            <strong>{count}</strong>
            {move || if is_even() {
                " is"
            } else {
                " is not"
            }}
            " even."
        </div>
    }
}
```

This kind of “provide a signal in a parent, consume it in a child” should be familiar
from the chapter on [parent-child interactions](./view/08_parent_child.md). The same
pattern you use to communicate between parents and children works for grandparents and
grandchildren, or any ancestors and descendants: in other words, between “global” state
in the root component of your app and any other components anywhere else in the app.

Because of the fine-grained nature of updates, this is usually all you need. However,
in some cases with more complex state changes, you may want to use a slightly more
structured approach to global state.

## Option #3: Create a Global State Struct

You can use this approach to build a single global data structure
that holds the state for your whole app, and then access it by
taking fine-grained slices using
[`create_slice`](https://docs.rs/leptos/latest/leptos/fn.create_slice.html)
or [`create_memo`](https://docs.rs/leptos/latest/leptos/fn.create_memo.html),
so that changing one part of the state doesn't cause parts of your
app that depend on other parts of the state to change.

You can begin by defining a simple state struct:

```rust
#[derive(Default, Clone, Debug)]
struct GlobalState {
    count: u32,
    name: String,
}
```

Provide it in the root of your app so it’s available everywhere.

```rust
#[component]
fn App(cx: Scope) -> impl IntoView {
    // we'll provide a single signal that holds the whole state
    // each component will be responsible for creating its own "lens" into it
    let state = create_rw_signal(cx, GlobalState::default());
    provide_context(cx, state);

    // ...
}
```

Then child components can access “slices” of that state with fine-grained
updates via `create_slice`. Each slice signal only updates when the particular
piece of the larger struct it accesses updates. This means you can create a single
root signal, and then take independent, fine-grained slices of it in different
components, each of which can update without notifying the others of changes.

```rust
/// A component that updates the count in the global state.
#[component]
fn GlobalStateCounter(cx: Scope) -> impl IntoView {
    let state = use_context::<RwSignal<GlobalState>>(cx).expect("state to have been provided");

    // `create_slice` lets us create a "lens" into the data
    let (count, set_count) = create_slice(
        cx,
        // we take a slice *from* `state`
        state,
        // our getter returns a "slice" of the data
        |state| state.count,
        // our setter describes how to mutate that slice, given a new value
        |state, n| state.count = n,
    );

    view! { cx,
        <div class="consumer blue">
            <button
                on:click=move |_| {
                    set_count(count() + 1);
                }
            >
                "Increment Global Count"
            </button>
            <br/>
            <span>"Count is: " {count}</span>
        </div>
    }
}
```

Clicking this button only updates `state.count`, so if we create another slice
somewhere else that only takes `state.name`, clicking the button won’t cause
that other slice to update. This allows you to combine the benefits of a top-down
data flow and of fine-grained reactive updates.

[Click to open CodeSandbox.](https://codesandbox.io/p/sandbox/1-basic-component-forked-8bte19?selection=%5B%7B%22endColumn%22%3A1%2C%22endLineNumber%22%3A2%2C%22startColumn%22%3A1%2C%22startLineNumber%22%3A2%7D%5D&file=%2Fsrc%2Fmain.rs)

<iframe src="https://codesandbox.io/p/sandbox/1-basic-component-forked-8bte19?selection=%5B%7B%22endColumn%22%3A1%2C%22endLineNumber%22%3A2%2C%22startColumn%22%3A1%2C%22startLineNumber%22%3A2%7D%5D&file=%2Fsrc%2Fmain.rs" width="100%" height="1000px" style="max-height: 100vh"></iframe>
