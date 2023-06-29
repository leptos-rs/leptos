# Iteration

Whether you’re listing todos, displaying a table, or showing product images,
iterating over a list of items is a common task in web applications. Reconciling
the differences between changing sets of items can also be one of the trickiest
tasks for a framework to handle well.

Leptos supports to two different patterns for iterating over items:

1. For static views: `Vec<_>`
2. For dynamic lists: `<For/>`

## Static Views with `Vec<_>`

Sometimes you need to show an item repeatedly, but the list you’re drawing from
does not often change. In this case, it’s important to know that you can insert
any `Vec<IV> where IV: IntoView` into your view. In other words, if you can render
`T`, you can render `Vec<T>`.

```rust
let values = vec![0, 1, 2];
view! { cx,
    // this will just render "012"
    <p>{values.clone()}</p>
    // or we can wrap them in <li>
    <ul>
        {values.into_iter()
            .map(|n| view! { cx, <li>{n}</li>})
            .collect::<Vec<_>>()}
    </ul>
}
```

Leptos also provides a `.collect_view(cx)` helper function that allows you to collect any iterator of `T: IntoView` into `Vec<View>`.

```rust
let values = vec![0, 1, 2];
view! { cx,
    // this will just render "012"
    <p>{values.clone()}</p>
    // or we can wrap them in <li>
    <ul>
        {values.into_iter()
            .map(|n| view! { cx, <li>{n}</li>})
            .collect_view(cx)}
    </ul>
}
```

The fact that the _list_ is static doesn’t mean the interface needs to be static.
You can render dynamic items as part of a static list.

```rust
// create a list of N signals
let counters = (1..=length).map(|idx| create_signal(cx, idx));

// each item manages a reactive view
// but the list itself will never change
let counter_buttons = counters
    .map(|(count, set_count)| {
        view! { cx,
            <li>
                <button
                    on:click=move |_| set_count.update(|n| *n += 1)
                >
                    {count}
                </button>
            </li>
        }
    })
    .collect_view(cx);

view! { cx,
    <ul>{counter_buttons}</ul>
}
```

You _can_ render a `Fn() -> Vec<_>` reactively as well. But note that every time
it changes, this will rerender every item in the list. This is quite inefficient!
Fortunately, there’s a better way.

## Dynamic Rendering with the `<For/>` Component

The [`<For/>`](https://docs.rs/leptos/latest/leptos/fn.For.html) component is a
keyed dynamic list. It takes three props:

- `each`: a function (such as a signal) that returns the items `T` to be iterated over
- `key`: a key function that takes `&T` and returns a stable, unique key or ID
- `view`: renders each `T` into a view

`key` is, well, the key. You can add, remove, and move items within the list. As
long as each item’s key is stable over time, the framework does not need to rerender
any of the items, unless they are new additions, and it can very efficiently add,
remove, and move items as they change. This allows for extremely efficient updates
to the list as it changes, with minimal additional work.

Creating a good `key` can be a little tricky. You generally do _not_ want to use
an index for this purpose, as it is not stable—if you remove or move items, their
indices change.

But it’s a great idea to do something like generating a unique ID for each row as
it is generated, and using that as an ID for the key function.

Check out the `<DynamicList/>` component below for an example.

[Click to open CodeSandbox.](https://codesandbox.io/p/sandbox/4-iteration-sglt1o?file=%2Fsrc%2Fmain.rs&selection=%5B%7B%22endColumn%22%3A6%2C%22endLineNumber%22%3A55%2C%22startColumn%22%3A5%2C%22startLineNumber%22%3A31%7D%5D)

<iframe src="https://codesandbox.io/p/sandbox/4-iteration-sglt1o?file=%2Fsrc%2Fmain.rs&selection=%5B%7B%22endColumn%22%3A6%2C%22endLineNumber%22%3A55%2C%22startColumn%22%3A5%2C%22startLineNumber%22%3A31%7D%5D" width="100%" height="1000px" style="max-height: 100vh"></iframe>
