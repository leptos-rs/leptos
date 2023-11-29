# Projecting Children

As you build components you may occasionally find yourself wanting to “project” children through multiple layers of components.

## The Problem

Consider the following:

```rust
pub fn LoggedIn<F, IV>(fallback: F, children: ChildrenFn) -> impl IntoView
where
    F: Fn() -> IV + 'static,
    IV: IntoView,
{
    view! {
        <Suspense
            fallback=|| ()
        >
            <Show
				// check whether user is verified
				// by reading from the resource
                when=move || todo!()
                fallback=fallback
            >
				{children()}
			</Show>
        </Suspense>
    }
}
```

This is pretty straightforward: when the user is logged in, we want to show `children`. If the user is not logged in, we want to show `fallback`. And while we’re waiting to find out, we just render `()`, i.e., nothing.

In other words, we want to pass the children of `<LoggedIn/>` _through_ the `<Suspense/>` component to become the children of the `<Show/>`. This is what I mean by “projection.”

This won’t compile.

```
error[E0507]: cannot move out of `fallback`, a captured variable in an `Fn` closure
error[E0507]: cannot move out of `children`, a captured variable in an `Fn` closure
```

The problem here is that both `<Suspense/>` and `<Show/>` need to be able to construct their `children` multiple times. The first time you construct `<Suspense/>`’s children, it would take ownership of `fallback` and `children` to move them into the invocation of `<Show/>`, but then they're not available for future `<Suspense/>` children construction.

## The Details

> Feel free to skip ahead to the solution.

If you want to really understand the issue here, it may help to look at the expanded `view` macro. Here’s a cleaned-up version:

```rust
Suspense(
    ::leptos::component_props_builder(&Suspense)
        .fallback(|| ())
        .children({
            // fallback and children are moved into this closure
            Box::new(move || {
                {
                    // fallback and children captured here
                    leptos::Fragment::lazy(|| {
                        vec![
                            (Show(
                                ::leptos::component_props_builder(&Show)
                                    .when(|| true)
									// but fallback is moved into Show here
                                    .fallback(fallback)
									// and children is moved into Show here
                                    .children(children)
                                    .build(),
                            )
                            .into_view()),
                        ]
                    })
                }
            })
        })
        .build(),
)
```

All components own their props; so the `<Show/>` in this case can’t be called because it only has captured references to `fallback` and `children`.

## Solution

However, both `<Suspense/>` and `<Show/>` take `ChildrenFn`, i.e., their `children` should implement the `Fn` type so they can be called multiple times with only an immutable reference. This means we don’t need to own `children` or `fallback`; we just need to be able to pass `'static` references to them.

We can solve this problem by using the [`store_value`](https://docs.rs/leptos/latest/leptos/fn.store_value.html) primitive. This essentially stores a value in the reactive system, handing ownership off to the framework in exchange for a reference that is, like signals, `Copy` and `'static`, which we can access or modify through certain methods.

In this case, it’s really simple:

```rust
pub fn LoggedIn<F, IV>(fallback: F, children: ChildrenFn) -> impl IntoView
where
    F: Fn() -> IV + 'static,
    IV: IntoView,
{
    let fallback = store_value(fallback);
    let children = store_value(children);
    view! {
        <Suspense
            fallback=|| ()
        >
            <Show
                when=|| todo!()
                fallback=move || fallback.with_value(|fallback| fallback())
            >
                {children.with_value(|children| children())}
            </Show>
        </Suspense>
    }
}
```

At the top level, we store both `fallback` and `children` in the reactive scope owned by `LoggedIn`. Now we can simply move those references down through the other layers into the `<Show/>` component and call them there.

## A Final Note

Note that this works because `<Show/>` and `<Suspense/>` only need an immutable reference to their children (which `.with_value` can give it), not ownership.

In other cases, you may need to project owned props through a function that takes `ChildrenFn` and therefore needs to be called more than once. In this case, you may find the `clone:` helper in the`view` macro helpful.

Consider this example

```rust
#[component]
pub fn App() -> impl IntoView {
    let name = "Alice".to_string();
    view! {
        <Outer>
            <Inner>
                <Inmost name=name.clone()/>
            </Inner>
        </Outer>
    }
}

#[component]
pub fn Outer(children: ChildrenFn) -> impl IntoView {
    children()
}

#[component]
pub fn Inner(children: ChildrenFn) -> impl IntoView {
    children()
}

#[component]
pub fn Inmost(name: String) -> impl IntoView {
    view! {
        <p>{name}</p>
    }
}
```

Even with `name=name.clone()`, this gives the error

```
cannot move out of `name`, a captured variable in an `Fn` closure
```

It’s captured through multiple levels of children that need to run more than once, and there’s no obvious way to clone it _into_ the children.

In this case, the `clone:` syntax comes in handy. Calling `clone:name` will clone `name` _before_ moving it into `<Inner/>`’s children, which solves our ownership issue.

```rust
view! {
	<Outer>
		<Inner clone:name>
			<Inmost name=name.clone()/>
		</Inner>
	</Outer>
}
```

These issues can be a little tricky to understand or debug, because of the opacity of the `view` macro. But in general, they can always be solved.
