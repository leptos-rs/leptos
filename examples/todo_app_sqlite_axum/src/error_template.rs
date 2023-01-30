use crate::errors::TodoAppError;
use http::status::StatusCode;
use leptos::Errors;
use leptos::{
    component, create_rw_signal, use_context, view, For, ForProps, IntoView, RwSignal, Scope,
};
use leptos_axum::ResponseOptions;
use miette::Diagnostic;

// A basic function to display errors served by the error boundaries. Feel free to do more complicated things
// here than just displaying them
#[component]
pub async fn ErrorTemplate(
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

    let response = use_context::<ResponseOptions>(cx).unwrap();
    response.set_status(StatusCode::from_u16(error_code.to_string().parse()).unwrap()).await;

    view! {cx,
    <h1>"Errors"</h1>
    <For
        // a function that returns the items we're iterating over; a signal is fine
        each= move || {errors.get().0}
        // a unique key for each item as a reference
        key=|error| error.0.clone()
        // renders each item to a view
        view= move |error| {
        let int_err = &*error.1.downcast_ref::<TodoAppError>().unwrap();
        let error_string = int_err.to_string();
        let error_code= int_err.code().unwrap();
          view! {
            cx,
            <h2>{error_code.to_string()}</h2>
            <p>"Error: " {error_string}</p>
          }
        }
      />
    }
}
