# Responding to Changes with `create_effect`

We’ve made it this far without having mentioned half of the reactive system: effects.

Reactivity works in two halves: updating individual reactive values (“signals”) notifies the pieces of code that depend on them (“effects”) that they need to run again. These two halves of the reactive system are inter-dependent. Without effects, signals can change within the reactive system but never be observed in a way that interacts with the outside world. Without signals, effects run once but never again, as there’s no observable value to subscribe to. Effects are quite literally “side effects” of the reactive system: they exist to synchronize the reactive system with the non-reactive world outside it.

Hidden behind the whole reactive DOM renderer that we’ve seen so far is a function called `create_effect`.

[`create_effect`](https://docs.rs/leptos_reactive/latest/leptos_reactive/fn.create_effect.html) takes a function as its argument. It immediately runs the function. If you access any reactive signal inside that function, it registers the fact that the effect depends on that signal with the reactive runtime. Whenever one of the signals that the effect depends on changes, the effect runs again.

```rust
let (a, set_a) = create_signal(0);
let (b, set_b) = create_signal(0);

create_effect(move |_| {
  // immediately prints "Value: 0" and subscribes to `a`
  log::debug!("Value: {}", a());
});
```

The effect function is called with an argument containing whatever value it returned the last time it ran. On the initial run, this is `None`.

By default, effects **do not run on the server**. This means you can call browser-specific APIs within the effect function without causing issues. If you need an effect to run on the server, use [`create_isomorphic_effect`](https://docs.rs/leptos_reactive/latest/leptos_reactive/fn.create_isomorphic_effect.html).

## Autotracking and Dynamic Dependencies

If you’re familiar with a framework like React, you might notice one key difference. React and similar frameworks typically require you to pass a “dependency array,” an explicit set of variables that determine when the effect should rerun.

Because Leptos comes from the tradition of synchronous reactive programming, we don’t need this explicit dependency list. Instead, we automatically track dependencies depending on which signals are accessed within the effect.

This has two effects (no pun intended). Dependencies are:

1. **Automatic**: You don’t need to maintain a dependency list, or worry about what should or shouldn’t be included. The framework simply tracks which signals might cause the effect to rerun, and handles it for you.
2. **Dynamic**: The dependency list is cleared and updated every time the effect runs. If your effect contains a conditional (for example), only signals that are used in the current branch are tracked. This means that effects rerun the absolute minimum number of times.

> If this sounds like magic, and if you want a deep dive into how automatic dependency tracking works, [check out this video](https://www.youtube.com/watch?v=GWB3vTWeLd4). (Apologies for the low volume!)

## Effects as Zero-Cost-ish Abstraction

While they’re not a “zero-cost abstraction” in the most technical sense—they require some additional memory use, exist at runtime, etc.—at a higher level, from the perspective of whatever expensive API calls or other work you’re doing within them, effects are a zero-cost abstraction. They rerun the absolute minimum number of times necessary, given how you’ve described them.

Imagine that I’m creating some kind of chat software, and I want people to be able to display their full name, or just their first name, and to notify the server whenever their name changes:

```rust
let (first, set_first) = create_signal(String::new());
let (last, set_last) = create_signal(String::new());
let (use_last, set_use_last) = create_signal(true);

// this will add the name to the log
// any time one of the source signals changes
create_effect(move |_| {
    log(
        if use_last() {
            format!("{} {}", first(), last())
        } else {
            first()
        },
    )
});
```

If `use_last` is `true`, effect should rerun whenever `first`, `last`, or `use_last` changes. But if I toggle `use_last` to `false`, a change in `last` will never cause the full name to change. In fact, `last` will be removed from the dependency list until `use_last` toggles again. This saves us from sending multiple unnecessary requests to the API if I change `last` multiple times while `use_last` is still `false`.

## To `create_effect`, or not to `create_effect`?

Effects are intended to run _side-effects_ of the system, not to synchronize state _within_ the system. In other words: don’t write to signals within effects.

If you need to define a signal that depends on the value of other signals, use a derived signal or [`create_memo`](https://docs.rs/leptos_reactive/latest/leptos_reactive/fn.create_memo.html).

If you need to synchronize some reactive value with the non-reactive world outside—like a web API, the console, the filesystem, or the DOM—create an effect.

> If you’re curious for more information about when you should and shouldn’t use `create_effect`, [check out this video](https://www.youtube.com/watch?v=aQOFJQ2JkvQ) for a more in-depth consideration!

## Effects and Rendering

We’ve managed to get this far without mentioning effects because they’re built into the Leptos DOM renderer. We’ve seen that you can create a signal and pass it into the `view` macro, and it will update the relevant DOM node whenever the signal changes:

```rust
let (count, set_count) = create_signal(0);

view! {
    <p>{count}</p>
}
```

This works because the framework essentially creates an effect wrapping this update. You can imagine Leptos translating this view into something like this:

```rust
let (count, set_count) = create_signal(0);

// create a DOM element
let p = create_element("p");

// create an effect to reactively update the text
create_effect(move |prev_value| {
    // first, access the signal’s value and convert it to a string
    let text = count().to_string();

    // if this is different from the previous value, update the node
    if prev_value != Some(text) {
        p.set_text_content(&text);
    }

    // return this value so we can memoize the next update
    text
});
```

Every time `count` is updated, this effect wil rerun. This is what allows reactive, fine-grained updates to the DOM.

## Explicit, Cancelable Tracking with `watch`

In addition to `create_effect`, Leptos provides a [`watch`](https://docs.rs/leptos_reactive/latest/leptos_reactive/fn.watch.html) function, which can be used for two main purposes:

1. Separating tracking and responding to changes by explicitly passing in a set of values to track.
2. Canceling tracking by calling a stop function.

Like `create_resource`, `watch` takes a first argument, which is reactively tracked, and a second, which is not. Whenever a reactive value in its `deps` argument is changed, the `callback` is run. `watch` returns a function that can be called to stop tracking the dependencies.

```rust
let (num, set_num) = create_signal(0);

let stop = watch(
    move || num.get(),
    move |num, prev_num, _| {
        log::debug!("Number: {}; Prev: {:?}", num, prev_num);
    },
    false,
);

set_num.set(1); // > "Number: 1; Prev: Some(0)"

stop(); // stop watching

set_num.set(2); // (nothing happens)
```

[Click to open CodeSandbox.](https://codesandbox.io/p/sandbox/14-effect-0-5-d6hkch?file=%2Fsrc%2Fmain.rs%3A1%2C1)

<iframe src="https://codesandbox.io/p/sandbox/14-effect-0-5-d6hkch?file=%2Fsrc%2Fmain.rs%3A1%2C1" width="100%" height="1000px" style="max-height: 100vh"></iframe>

<details>
<summary>CodeSandbox Source</summary>

```rust
use leptos::html::Input;
use leptos::*;

#[derive(Copy, Clone)]
struct LogContext(RwSignal<Vec<String>>);

#[component]
fn App() -> impl IntoView {
    // Just making a visible log here
    // You can ignore this...
    let log = create_rw_signal::<Vec<String>>(vec![]);
    let logged = move || log().join("\n");

    // the newtype pattern isn't *necessary* here but is a good practice
    // it avoids confusion with other possible future `RwSignal<Vec<String>>` contexts
    // and makes it easier to refer to it
    provide_context(LogContext(log));

    view! {
        <CreateAnEffect/>
        <pre>{logged}</pre>
    }
}

#[component]
fn CreateAnEffect() -> impl IntoView {
    let (first, set_first) = create_signal(String::new());
    let (last, set_last) = create_signal(String::new());
    let (use_last, set_use_last) = create_signal(true);

    // this will add the name to the log
    // any time one of the source signals changes
    create_effect(move |_| {
        log(if use_last() {
            with!(|first, last| format!("{first} {last}"))
        } else {
            first()
        })
    });

    view! {
        <h1>
            <code>"create_effect"</code>
            " Version"
        </h1>
        <form>
            <label>
                "First Name"
                <input
                    type="text"
                    name="first"
                    prop:value=first
                    on:change=move |ev| set_first(event_target_value(&ev))
                />
            </label>
            <label>
                "Last Name"
                <input
                    type="text"
                    name="last"
                    prop:value=last
                    on:change=move |ev| set_last(event_target_value(&ev))
                />
            </label>
            <label>
                "Show Last Name"
                <input
                    type="checkbox"
                    name="use_last"
                    prop:checked=use_last
                    on:change=move |ev| set_use_last(event_target_checked(&ev))
                />
            </label>
        </form>
    }
}

#[component]
fn ManualVersion() -> impl IntoView {
    let first = create_node_ref::<Input>();
    let last = create_node_ref::<Input>();
    let use_last = create_node_ref::<Input>();

    let mut prev_name = String::new();
    let on_change = move |_| {
        log("      listener");
        let first = first.get().unwrap();
        let last = last.get().unwrap();
        let use_last = use_last.get().unwrap();
        let this_one = if use_last.checked() {
            format!("{} {}", first.value(), last.value())
        } else {
            first.value()
        };

        if this_one != prev_name {
            log(&this_one);
            prev_name = this_one;
        }
    };

    view! {
        <h1>"Manual Version"</h1>
        <form on:change=on_change>
            <label>"First Name" <input type="text" name="first" node_ref=first/></label>
            <label>"Last Name" <input type="text" name="last" node_ref=last/></label>
            <label>
                "Show Last Name" <input type="checkbox" name="use_last" checked node_ref=use_last/>
            </label>
        </form>
    }
}

#[component]
fn EffectVsDerivedSignal() -> impl IntoView {
    let (my_value, set_my_value) = create_signal(String::new());
    // Don't do this.
    /*let (my_optional_value, set_optional_my_value) = create_signal(Option::<String>::None);

    create_effect(move |_| {
        if !my_value.get().is_empty() {
            set_optional_my_value(Some(my_value.get()));
        } else {
            set_optional_my_value(None);
        }
    });*/

    // Do this
    let my_optional_value =
        move || (!my_value.with(String::is_empty)).then(|| Some(my_value.get()));

    view! {
        <input prop:value=my_value on:input=move |ev| set_my_value(event_target_value(&ev))/>

        <p>
            <code>"my_optional_value"</code>
            " is "
            <code>
                <Show when=move || my_optional_value().is_some() fallback=|| view! { "None" }>
                    "Some(\""
                    {my_optional_value().unwrap()}
                    "\")"
                </Show>
            </code>
        </p>
    }
}

#[component]
pub fn Show<F, W, IV>(
    /// The components Show wraps
    children: Box<dyn Fn() -> Fragment>,
    /// A closure that returns a bool that determines whether this thing runs
    when: W,
    /// A closure that returns what gets rendered if the when statement is false
    fallback: F,
) -> impl IntoView
where
    W: Fn() -> bool + 'static,
    F: Fn() -> IV + 'static,
    IV: IntoView,
{
    let memoized_when = create_memo(move |_| when());

    move || match memoized_when.get() {
        true => children().into_view(),
        false => fallback().into_view(),
    }
}

fn log(msg: impl std::fmt::Display) {
    let log = use_context::<LogContext>().unwrap().0;
    log.update(|log| log.push(msg.to_string()));
}

fn main() {
    leptos::mount_to_body(App)
}
```

</details>
</preview>
