use http::status::StatusCode;
use leptos::Errors;
use leptos::{component, create_rw_signal, view, For, ForProps, IntoView, RwSignal, Scope};

// A basic function to display errors served by the error boundaries. Feel free to do more complicated things
// here than just displaying them
#[component]
pub fn ErrorTemplate(
    cx: Scope,
    #[prop(optional)] outside_errors: Option<Errors>,
    #[prop(optional)] errors: Option<RwSignal<Errors>>,
) -> impl IntoView {
    let errors = match outside_errors {
        Some(e) => {
            let errors = create_rw_signal(cx, e);
            errors
        }
        None => match errors {
            Some(e) => e,
            None => panic!("No Errors found and we expected errors!"),
        },
    };

    view! {cx,
    <h1>"Errors"</h1>
    <For
        // a function that returns the items we're iterating over; a signal is fine
        each= move || {errors.get().0.into_iter()}
        // a unique key for each item as a reference
        key=|error| error.0.clone()
        // renders each item to a view
        view= move |error| {
        let error_string = error.1.to_string();
        let error_code= error.1.code().unwrap();
          view! {
            cx,
            <h2>{error_code.to_string()}</h2>
            <p>"Error: " {error_string}</p>
          }
        }
      />
    }
}
