# Component Children

It’s pretty common to want to pass children into a component, just as you can pass
children into an HTML element. For example, imagine I have a `<FancyForm/>` component
that enhances an HTML `<form>`. I need some way to pass all its inputs.

```rust
view! {
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
view! {
  <Show
    // `when` is a normal prop
    when=move || value() > 5
    // `fallback` is a "render prop": a function that returns a view
    fallback=|| view! { <Small/> }
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
    view! {
        <h2>"Render Prop"</h2>
        {render_prop()}

        <h2>"Children"</h2>
        {children()}
    }
}
```

`render_prop` and `children` are both functions, so we can call them to generate
the appropriate views. `children`, in particular, is an alias for
`Box<dyn FnOnce() -> Fragment>`. (Aren't you glad we named it `Children` instead?)

> If you need a `Fn` or `FnMut` here because you need to call `children` more than once,
> we also provide `ChildrenFn` and `ChildrenMut` aliases.

We can use the component like this:

```rust
view! {
    <TakesChildren render_prop=|| view! { <p>"Hi, there!"</p> }>
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
pub fn WrapsChildren(children: Children) -> impl IntoView {
    // Fragment has `nodes` field that contains a Vec<View>
    let children = children()
        .nodes
        .into_iter()
        .map(|child| view! { <li>{child}</li> })
        .collect_view();

    view! {
        <ul>{children}</ul>
    }
}
```

Calling it like this will create a list:

```rust
view! {
    <WrapsChildren>
        "A"
        "B"
        "C"
    </WrapsChildren>
}
```

[Click to open CodeSandbox.](https://codesandbox.io/p/sandbox/9-component-children-0-5-m4jwhp?file=%2Fsrc%2Fmain.rs%3A1%2C1)

<iframe src="https://codesandbox.io/p/sandbox/9-component-children-0-5-m4jwhp?file=%2Fsrc%2Fmain.rs%3A1%2C1" width="100%" height="1000px" style="max-height: 100vh"></iframe>

<details>
<summary>CodeSandbox Source</summary>

```rust
use leptos::*;

// Often, you want to pass some kind of child view to another
// component. There are two basic patterns for doing this:
// - "render props": creating a component prop that takes a function
//   that creates a view
// - the `children` prop: a special property that contains content
//   passed as the children of a component in your view, not as a
//   property

#[component]
pub fn App() -> impl IntoView {
    let (items, set_items) = create_signal(vec![0, 1, 2]);
    let render_prop = move || {
        // items.with(...) reacts to the value without cloning
        // by applying a function. Here, we pass the `len` method
        // on a `Vec<_>` directly
        let len = move || items.with(Vec::len);
        view! {
            <p>"Length: " {len}</p>
        }
    };

    view! {
        // This component just displays the two kinds of children,
        // embedding them in some other markup
        <TakesChildren
            // for component props, you can shorthand
            // `render_prop=render_prop` => `render_prop`
            // (this doesn't work for HTML element attributes)
            render_prop
        >
            // these look just like the children of an HTML element
            <p>"Here's a child."</p>
            <p>"Here's another child."</p>
        </TakesChildren>
        <hr/>
        // This component actually iterates over and wraps the children
        <WrapsChildren>
            <p>"Here's a child."</p>
            <p>"Here's another child."</p>
        </WrapsChildren>
    }
}

/// Displays a `render_prop` and some children within markup.
#[component]
pub fn TakesChildren<F, IV>(
    /// Takes a function (type F) that returns anything that can be
    /// converted into a View (type IV)
    render_prop: F,
    /// `children` takes the `Children` type
    /// this is an alias for `Box<dyn FnOnce() -> Fragment>`
    /// ... aren't you glad we named it `Children` instead?
    children: Children,
) -> impl IntoView
where
    F: Fn() -> IV,
    IV: IntoView,
{
    view! {
        <h1><code>"<TakesChildren/>"</code></h1>
        <h2>"Render Prop"</h2>
        {render_prop()}
        <hr/>
        <h2>"Children"</h2>
        {children()}
    }
}

/// Wraps each child in an `<li>` and embeds them in a `<ul>`.
#[component]
pub fn WrapsChildren(children: Children) -> impl IntoView {
    // children() returns a `Fragment`, which has a
    // `nodes` field that contains a Vec<View>
    // this means we can iterate over the children
    // to create something new!
    let children = children()
        .nodes
        .into_iter()
        .map(|child| view! { <li>{child}</li> })
        .collect::<Vec<_>>();

    view! {
        <h1><code>"<WrapsChildren/>"</code></h1>
        // wrap our wrapped children in a UL
        <ul>{children}</ul>
    }
}

fn main() {
    leptos::mount_to_body(App)
}
```

</details>
</preview>
