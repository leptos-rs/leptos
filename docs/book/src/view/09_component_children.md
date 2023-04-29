# Component Children

It’s pretty common to want to pass children into a component, just as you can pass
children into an HTML element. For example, imagine I have a `<FancyForm/>` component
that enhances an HTML `<form>`. I need some way to pass all its inputs.

```rust
view! { cx,
    <Form>
        <fieldset>
            <label>
                "Some Input"
                <input type="text" name="something"/>
            </label>
        </fieldset>
        <button>"Submit"</button>
    </Form>
}
```

How can you do this in Leptos? There are basically two ways to pass components to
other components:

1. **render props**: properties that are functions that return a view
2. the **`children`** prop: a special component property that includes anything
   you pass as a child to the component.

In fact, you’ve already seen these both in action in the [`<Show/>`](/view/06_control_flow.html#show) component:

```rust
view! { cx,
  <Show
    // `when` is a normal prop
    when=move || value() > 5
    // `fallback` is a "render prop": a function that returns a view
    fallback=|cx| view! { cx, <Small/> }
  >
    // `<Big/>` (and anything else here)
    // will be given to the `children` prop
    <Big/>
  </Show>
}
```

Let’s define a component that takes some children and a render prop.

```rust
#[component]
pub fn TakesChildren<F, IV>(
    cx: Scope,
    /// Takes a function (type F) that returns anything that can be
    /// converted into a View (type IV)
    render_prop: F,
    /// `children` takes the `Children` type
    children: Children,
) -> impl IntoView
where
    F: Fn() -> IV,
    IV: IntoView,
{
    view! { cx,
        <h2>"Render Prop"</h2>
        {render_prop()}

        <h2>"Children"</h2>
        {children(cx)}
    }
}
```

`render_prop` and `children` are both functions, so we can call them to generate
the appropriate views. `children`, in particular, is an alias for
`Box<dyn FnOnce(Scope) -> Fragment>`. (Aren't you glad we named it `Children` instead?)

> If you need a `Fn` or `FnMut` here because you need to call `children` more than once,
> we also provide `ChildrenFn` and `ChildrenMut` aliases.

We can use the component like this:

```rust
view! { cx,
    <TakesChildren render_prop=|| view! { cx, <p>"Hi, there!"</p> }>
        // these get passed to `children`
        "Some text"
        <span>"A span"</span>
    </TakesChildren>
}
```

## Manipulating Children

The [`Fragment`](https://docs.rs/leptos/latest/leptos/struct.Fragment.html) type is
basically a way of wrapping a `Vec<View>`. You can insert it anywhere into your view.

But you can also access those inner views directly to manipulate them. For example, here’s
a component that takes its children and turns them into an unordered list.

```rust
#[component]
pub fn WrapsChildren(cx: Scope, children: Children) -> impl IntoView {
    // Fragment has `nodes` field that contains a Vec<View>
    let children = children(cx)
        .nodes
        .into_iter()
        .map(|child| view! { cx, <li>{child}</li> })
        .collect_view(cx);

    view! { cx,
        <ul>{children}</ul>
    }
}
```

Calling it like this will create a list:

```rust
view! { cx,
    <WrappedChildren>
        "A"
        "B"
        "C"
    </WrappedChildren>
}
```

[Click to open CodeSandbox.](https://codesandbox.io/p/sandbox/9-component-children-2wrdfd?file=%2Fsrc%2Fmain.rs&selection=%5B%7B%22endColumn%22%3A12%2C%22endLineNumber%22%3A19%2C%22startColumn%22%3A12%2C%22startLineNumber%22%3A19%7D%5D)

<iframe src="https://codesandbox.io/p/sandbox/9-component-children-2wrdfd?file=%2Fsrc%2Fmain.rs&selection=%5B%7B%22endColumn%22%3A12%2C%22endLineNumber%22%3A19%2C%22startColumn%22%3A12%2C%22startLineNumber%22%3A19%7D%5D" width="100%" height="1000px" style="max-height: 100vh"></iframe>
