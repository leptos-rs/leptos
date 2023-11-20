# Global State Management

So far, we've only been working with local state in components, and we’ve seen how to coordinate state between parent and child components. On occasion, there are times where people look for a more general solution for global state management that can work throughout an application.

In general, **you do not need this chapter.** The typical pattern is to compose your application out of components, each of which manages its own local state, not to store all state in a global structure. However, there are some cases (like theming, saving user settings, or sharing data between components in different parts of your UI) in which you may want to use some kind of global state management.

The three best approaches to global state are

1. Using the router to drive global state via the URL
2. Passing signals through context
3. Creating a global state struct and creating lenses into it with `create_slice`

## Option #1: URL as Global State

In many ways, the URL is actually the best way to store global state. It can be accessed from any component, anywhere in your tree. There are native HTML elements like `<form>` and `<a>` that exist solely to update the URL. And it persists across page reloads and between devices; you can share a URL with a friend or send it from your phone to your laptop and any state stored in it will be replicated.

The next few sections of the tutorial will be about the router, and we’ll get much more into these topics.

But for now, we'll just look at options #2 and #3.

## Option #2: Passing Signals through Context

In the section on [parent-child communication](view/08_parent_child.md), we saw that you can use `provide_context` to pass signal from a parent component to a child, and `use_context` to read it in the child. But `provide_context` works across any distance. If you want to create a global signal that holds some piece of state, you can provide it and access it via context anywhere in the descendants of the component where you provide it.

A signal provided via context only causes reactive updates where it is read, not in any of the components in between, so it maintains the power of fine-grained reactive updates, even at a distance.

We start by creating a signal in the root of the app and providing it to
all its children and descendants using `provide_context`.

```rust
#[component]
fn App() -> impl IntoView {
    // here we create a signal in the root that can be consumed
    // anywhere in the app.
    let (count, set_count) = create_signal(0);
    // we'll pass the setter to specific components,
    // but provide the count itself to the whole app via context
    provide_context(count);

    view! {
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
fn FancyMath() -> impl IntoView {
    // here we consume the global count signal with `use_context`
    let count = use_context::<ReadSignal<u32>>()
        // we know we just provided this in the parent component
        .expect("there to be a `count` signal provided");
    let is_even = move || count() & 1 == 0;

    view! {
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

Note that this same pattern can be applied to more complex state. If you have multiple fields you want to update independently, you can do that by providing some struct of signals:

```rust
#[derive(Copy, Clone, Debug)]
struct GlobalState {
    count: RwSignal<i32>,
    name: RwSignal<String>
}

impl GlobalState {
    pub fn new() -> Self {
        Self {
            count: create_rw_signal(0),
            name: create_rw_signal("Bob".to_string())
        }
    }
}

#[component]
fn App() -> impl IntoView {
    provide_context(GlobalState::new());

    // etc.
}
```

## Option #3: Create a Global State Struct and Slices

You may find it cumbersome to wrap each field of a structure in a separate signal like this. In some cases, it can be useful to create a plain struct with non-reactive fields, and then wrap that in a signal.

```rust
#[derive(Copy, Clone, Debug, Default)]
struct GlobalState {
    count: i32,
    name: String
}

#[component]
fn App() -> impl IntoView {
    provide_context(create_rw_signal(GlobalState::default()));

    // etc.
}
```

But there’s a problem: because our whole state is wrapped in one signal, updating the value of one field will cause reactive updates in parts of the UI that only depend on the other.

```rust
let state = expect_context::<RwSignal<GlobalState>>();
view! {
    <button on:click=move |_| state.update(|n| *n += 1)>"+1"</button>
    <p>{move || state.with(|state| state.name.clone())}</p>
}
```

In this example, clicking the button will cause the text inside `<p>` to be updated, cloning `state.name` again! Because signals are the atomic unit of reactivity, updating any field of the signal triggers updates to everything that depends on the signal.

There’s a better way. You can take fine-grained, reactive slices by using [`create_memo`](https://docs.rs/leptos/latest/leptos/fn.create_memo.html) or [`create_slice`](https://docs.rs/leptos/latest/leptos/fn.create_slice.html) (which uses `create_memo` but also provides a setter). “Memoizing” a value means creating a new reactive value which will only update when it changes. “Memoizing a slice” means creating a new reactive value which will only update when some field of the state struct updates.

Here, instead of reading from the state signal directly, we create “slices” of that state with fine-grained updates via `create_slice`. Each slice signal only updates when the particular piece of the larger struct it accesses updates. This means you can create a single root signal, and then take independent, fine-grained slices of it in different components, each of which can update without notifying the others of changes.

```rust
/// A component that updates the count in the global state.
#[component]
fn GlobalStateCounter() -> impl IntoView {
    let state = expect_context::<RwSignal<GlobalState>>();

    // `create_slice` lets us create a "lens" into the data
    let (count, set_count) = create_slice(

        // we take a slice *from* `state`
        state,
        // our getter returns a "slice" of the data
        |state| state.count,
        // our setter describes how to mutate that slice, given a new value
        |state, n| state.count = n,
    );

    view! {
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

> **Note**: There are some significant drawbacks to this approach. Both signals and memos need to own their values, so a memo will need to clone the field’s value on every change. The most natural way to manage state in a framework like Leptos is always to provide signals that are as locally-scoped and fine-grained as they can be, not to hoist everything up into global state. But when you _do_ need some kind of global state, `create_slice` can be a useful tool.

[Click to open CodeSandbox.](https://codesandbox.io/p/sandbox/15-global-state-0-5-8c2ff6?file=%2Fsrc%2Fmain.rs%3A1%2C2)

<iframe src="https://codesandbox.io/p/sandbox/15-global-state-0-5-8c2ff6?file=%2Fsrc%2Fmain.rs%3A1%2C2" width="100%" height="1000px" style="max-height: 100vh"></iframe>

<details>
<summary>CodeSandbox Source</summary>

```rust
use leptos::*;

// So far, we've only been working with local state in components
// We've only seen how to communicate between parent and child components
// But there are also more general ways to manage global state
//
// The three best approaches to global state are
// 1. Using the router to drive global state via the URL
// 2. Passing signals through context
// 3. Creating a global state struct and creating lenses into it with `create_slice`
//
// Option #1: URL as Global State
// The next few sections of the tutorial will be about the router.
// So for now, we'll just look at options #2 and #3.

// Option #2: Pass Signals through Context
//
// In virtual DOM libraries like React, using the Context API to manage global
// state is a bad idea: because the entire app exists in a tree, changing
// some value provided high up in the tree can cause the whole app to render.
//
// In fine-grained reactive libraries like Leptos, this is simply not the case.
// You can create a signal in the root of your app and pass it down to other
// components using provide_context(). Changing it will only cause rerendering
// in the specific places it is actually used, not the whole app.
#[component]
fn Option2() -> impl IntoView {
    // here we create a signal in the root that can be consumed
    // anywhere in the app.
    let (count, set_count) = create_signal(0);
    // we'll pass the setter to specific components,
    // but provide the count itself to the whole app via context
    provide_context(count);

    view! {
        <h1>"Option 2: Passing Signals"</h1>
        // SetterButton is allowed to modify the count
        <SetterButton set_count/>
        // These consumers can only read from it
        // But we could give them write access by passing `set_count` if we wanted
        <div style="display: flex">
            <FancyMath/>
            <ListItems/>
        </div>
    }
}

/// A button that increments our global counter.
#[component]
fn SetterButton(set_count: WriteSignal<u32>) -> impl IntoView {
    view! {
        <div class="provider red">
            <button on:click=move |_| set_count.update(|count| *count += 1)>
                "Increment Global Count"
            </button>
        </div>
    }
}

/// A component that does some "fancy" math with the global count
#[component]
fn FancyMath() -> impl IntoView {
    // here we consume the global count signal with `use_context`
    let count = use_context::<ReadSignal<u32>>()
        // we know we just provided this in the parent component
        .expect("there to be a `count` signal provided");
    let is_even = move || count() & 1 == 0;

    view! {
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

/// A component that shows a list of items generated from the global count.
#[component]
fn ListItems() -> impl IntoView {
    // again, consume the global count signal with `use_context`
    let count = use_context::<ReadSignal<u32>>().expect("there to be a `count` signal provided");

    let squares = move || {
        (0..count())
            .map(|n| view! { <li>{n}<sup>"2"</sup> " is " {n * n}</li> })
            .collect::<Vec<_>>()
    };

    view! {
        <div class="consumer green">
            <ul>{squares}</ul>
        </div>
    }
}

// Option #3: Create a Global State Struct
//
// You can use this approach to build a single global data structure
// that holds the state for your whole app, and then access it by
// taking fine-grained slices using `create_slice` or `create_memo`,
// so that changing one part of the state doesn't cause parts of your
// app that depend on other parts of the state to change.

#[derive(Default, Clone, Debug)]
struct GlobalState {
    count: u32,
    name: String,
}

#[component]
fn Option3() -> impl IntoView {
    // we'll provide a single signal that holds the whole state
    // each component will be responsible for creating its own "lens" into it
    let state = create_rw_signal(GlobalState::default());
    provide_context(state);

    view! {
        <h1>"Option 3: Passing Signals"</h1>
        <div class="red consumer" style="width: 100%">
            <h2>"Current Global State"</h2>
            <pre>
                {move || {
                    format!("{:#?}", state.get())
                }}
            </pre>
        </div>
        <div style="display: flex">
            <GlobalStateCounter/>
            <GlobalStateInput/>
        </div>
    }
}

/// A component that updates the count in the global state.
#[component]
fn GlobalStateCounter() -> impl IntoView {
    let state = use_context::<RwSignal<GlobalState>>().expect("state to have been provided");

    // `create_slice` lets us create a "lens" into the data
    let (count, set_count) = create_slice(

        // we take a slice *from* `state`
        state,
        // our getter returns a "slice" of the data
        |state| state.count,
        // our setter describes how to mutate that slice, given a new value
        |state, n| state.count = n,
    );

    view! {
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

/// A component that updates the count in the global state.
#[component]
fn GlobalStateInput() -> impl IntoView {
    let state = use_context::<RwSignal<GlobalState>>().expect("state to have been provided");

    // this slice is completely independent of the `count` slice
    // that we created in the other component
    // neither of them will cause the other to rerun
    let (name, set_name) = create_slice(
        // we take a slice *from* `state`
        state,
        // our getter returns a "slice" of the data
        |state| state.name.clone(),
        // our setter describes how to mutate that slice, given a new value
        |state, n| state.name = n,
    );

    view! {
        <div class="consumer green">
            <input
                type="text"
                prop:value=name
                on:input=move |ev| {
                    set_name(event_target_value(&ev));
                }
            />
            <br/>
            <span>"Name is: " {name}</span>
        </div>
    }
}
// This `main` function is the entry point into the app
// It just mounts our component to the <body>
// Because we defined it as `fn App`, we can now use it in a
// template as <App/>
fn main() {
    leptos::mount_to_body(|| view! { <Option2/><Option3/> })
}
```

</details>
</preview>
