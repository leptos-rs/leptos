use leptos::{component, create_signal, prelude::*, view, IntoView};

#[component]
pub fn App() -> impl IntoView {
    let (value, set_value) = create_signal(Ok(0));//"".parse::<i32>());
    let guard = value.read();

    view! {
            <h1>"Error Handling"</h1>
            <label>
                "Type a number (or something that's not a number!)"
                <input
                    type="text"
                    value=move || value.get().map(|n| n.to_string()).unwrap_or_default()// TODO guard support here
                    // when input changes, try to parse a number from the input
                    on:input:target=move |ev| set_value.set(ev.target().value().parse::<i32>())
                />

                // If an `Err(_) had been rendered inside the <ErrorBoundary/>,
                // the fallback will be displayed. Otherwise, the children of the
                // <ErrorBoundary/> will be displayed.
    /*            <ErrorBoundary
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
                                    .collect_view()
                                }
                            </ul>
                        </div>
                    }
                >*/
                    {move || view! {
                        <p>
                            "You entered "
                            // because `value` is `Result<i32, _>`,
                            // it will render the `i32` if it is `Ok`,
                            // and render nothing and trigger the error boundary
                            // if it is `Err`. It's a signal, so this will dynamically
                            // update when `value` changes
                            <strong>{move || value.get()}</strong>
                        </p>}
                        .catch(|e| view! {
                            <p class="error">{e.to_string()}</p>
                        })
                    }
                //</ErrorBoundary>
            </label>
        }
}

#[component]
pub fn ErrorBoundary() -> impl IntoView {}
