# Error Handling

[In the last chapter](./06_control_flow.md), we saw that you can render `Option<T>`:
in the `None` case, it will render nothing, and in the `T` case, it will render `T`
(that is, if `T` implements `IntoView`). You can actually do something very similar
with a `Result<T, E>`. In the `Err(_)` case, it will render nothing. In the `Ok(T)`
case, it will render the `T`.

Let’s start with a simple component to capture a number input.

```rust
#[component]
fn NumericInput() -> impl IntoView {
    let (value, set_value) = create_signal(Ok(0));

    // when input changes, try to parse a number from the input
    let on_input = move |ev| set_value(event_target_value(&ev).parse::<i32>());

    view! {
        <label>
            "Type a number (or not!)"
            <input type="number" on:input=on_input/>
            <p>
                "You entered "
                <strong>{value}</strong>
            </p>
        </label>
    }
}
```

Every time you change the input, `on_input` will attempt to parse its value into a 32-bit
integer (`i32`), and store it in our `value` signal, which is a `Result<i32, _>`. If you
type the number `42`, the UI will display

```
You entered 42
```

But if you type the string`foo`, it will display

```
You entered
```

This is not great. It saves us using `.unwrap_or_default()` or something, but it would be
much nicer if we could catch the error and do something with it.

You can do that, with the [`<ErrorBoundary/>`](https://docs.rs/leptos/latest/leptos/fn.ErrorBoundary.html)
component.

## `<ErrorBoundary/>`

An `<ErrorBoundary/>` is a little like the `<Show/>` component we saw in the last chapter.
If everything’s okay—which is to say, if everything is `Ok(_)`—it renders its children.
But if there’s an `Err(_)` rendered among those children, it will trigger the
`<ErrorBoundary/>`’s `fallback`.

Let’s add an `<ErrorBoundary/>` to this example.

```rust
#[component]
fn NumericInput() -> impl IntoView {
    let (value, set_value) = create_signal(Ok(0));

    let on_input = move |ev| set_value(event_target_value(&ev).parse::<i32>());

    view! {
        <h1>"Error Handling"</h1>
        <label>
            "Type a number (or something that's not a number!)"
            <input type="number" on:input=on_input/>
            <ErrorBoundary
                // the fallback receives a signal containing current errors
                fallback=|errors| view! {
                    <div class="error">
                        <p>"Not a number! Errors: "</p>
                        // we can render a list of errors as strings, if we'd like
                        <ul>
                            {move || errors.get()
                                .into_iter()
                                .map(|(_, e)| view! { <li>{e.to_string()}</li>})
                                .collect_view()
                            }
                        </ul>
                    </div>
                }
            >
                <p>"You entered " <strong>{value}</strong></p>
            </ErrorBoundary>
        </label>
    }
}
```

Now, if you type `42`, `value` is `Ok(42)` and you’ll see

```
You entered 42
```

If you type `foo`, value is `Err(_)` and the `fallback` will render. We’ve chosen to render
the list of errors as a `String`, so you’ll see something like

```
Not a number! Errors:
- cannot parse integer from empty string
```

If you fix the error, the error message will disappear and the content you’re wrapping in
an `<ErrorBoundary/>` will appear again.

[Click to open CodeSandbox.](https://codesandbox.io/p/sandbox/7-errors-0-5-5mptv9?file=%2Fsrc%2Fmain.rs%3A1%2C1)

<iframe src="https://codesandbox.io/p/sandbox/7-errors-0-5-5mptv9?file=%2Fsrc%2Fmain.rs%3A1%2C1" width="100%" height="1000px" style="max-height: 100vh"></iframe>

<details>
<summary>CodeSandbox Source</summary>

```rust
use leptos::*;

#[component]
fn App() -> impl IntoView {
    let (value, set_value) = create_signal(Ok(0));

    // when input changes, try to parse a number from the input
    let on_input = move |ev| set_value(event_target_value(&ev).parse::<i32>());

    view! {
        <h1>"Error Handling"</h1>
        <label>
            "Type a number (or something that's not a number!)"
            <input type="number" on:input=on_input/>
            // If an `Err(_) had been rendered inside the <ErrorBoundary/>,
            // the fallback will be displayed. Otherwise, the children of the
            // <ErrorBoundary/> will be displayed.
            <ErrorBoundary
                // the fallback receives a signal containing current errors
                fallback=|errors| view! {
                    <div class="error">
                        <p>"Not a number! Errors: "</p>
                        // we can render a list of errors
                        // as strings, if we'd like
                        <ul>
                            {move || errors.get()
                                .into_iter()
                                .map(|(_, e)| view! { <li>{e.to_string()}</li>})
                                .collect::<Vec<_>>()
                            }
                        </ul>
                    </div>
                }
            >
                <p>
                    "You entered "
                    // because `value` is `Result<i32, _>`,
                    // it will render the `i32` if it is `Ok`,
                    // and render nothing and trigger the error boundary
                    // if it is `Err`. It's a signal, so this will dynamically
                    // update when `value` changes
                    <strong>{value}</strong>
                </p>
            </ErrorBoundary>
        </label>
    }
}

fn main() {
    leptos::mount_to_body(App)
}
```

</details>
</preview>
