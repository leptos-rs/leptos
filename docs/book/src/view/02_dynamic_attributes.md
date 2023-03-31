# `view`: Dynamic Attributes and Classes

So far we’ve seen how to use the `view` macro to create event listeners and to 
create dynamic text by passing a function (such as a signal) into the view.

But of course there are other things you might want to update in your user interface.
In this section, we’ll look at how to update attributes and classes dynamically, 
and we’ll introduce the concept of a **derived signal**.

Let’s start with a simple component that should be familiar: click a button to 
increment a counter.

```rust
#[component]
fn App(cx: Scope) -> impl IntoView {
    let (count, set_count) = create_signal(cx, 0);

    view! { cx,
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

> Note: Using a derived signal like this means that the calculation runs once per 
signal change per place we access `double_count`; in other words, twice. This is a
very cheap calculation, so that’s fine. We’ll look at memos in a later chapter, which 
are designed to solve this problem for expensive calculations.

<iframe src="https://codesandbox.io/p/sandbox/2-dynamic-attribute-pqyvzl?file=%2Fsrc%2Fmain.rs&selection=%5B%7B%22endColumn%22%3A1%2C%22endLineNumber%22%3A2%2C%22startColumn%22%3A1%2C%22startLineNumber%22%3A2%7D%5D" width="100%" height="1000px"></iframe>
