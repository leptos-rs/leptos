use leptos::prelude::*;

#[component]
pub fn App() -> impl IntoView {
    let (value, set_value) = signal("".parse::<i32>());

    view! {
        <h1>"Error Handling"</h1>
        <label>
            "Type an integer (or something that's not an integer!)"
            <input
                type="number"
                value=move || value.get().unwrap_or_default()
                // when input changes, try to parse a number from the input
                on:input:target=move |ev| set_value.set(ev.target().value().parse::<i32>())
            />
            // If an `Err(_) has been rendered inside the <ErrorBoundary/>,
            // the fallback will be displayed. Otherwise, the children of the
            // <ErrorBoundary/> will be displayed.
            // the fallback receives a signal containing current errors
            <ErrorBoundary fallback=|errors| {
                let errors = errors.clone();
                view! {
                    <div class="error">
                        <p>"Not an integer! Errors: "</p>
                        // we can render a list of errors
                        // as strings, if we'd like
                        <ul>
                            {move || {
                                errors
                                    .read()
                                    .iter()
                                    .map(|(_, e)| view! { <li>{e.to_string()}</li> })
                                    .collect::<Vec<_>>()
                            }}

                        </ul>
                    </div>
                }
            }>

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
