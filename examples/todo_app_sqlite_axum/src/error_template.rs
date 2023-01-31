use crate::errors::TodoAppError;
use cfg_if::cfg_if;
use http::status::StatusCode;
use leptos::Errors;
use leptos::{
    component, create_rw_signal, use_context, view, For, ForProps, IntoView, RwSignal, Scope,
};
#[cfg(feature = "ssr")]
use leptos_axum::ResponseOptions;
use miette::Diagnostic;

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

    // Get Errors from Signal
    let errors = errors.get().0;

    // Downcast lets us take a type that implements `std::error::Error` and ask if it is
    let errors: Vec<TodoAppError> = errors
        .into_iter()
        .map(|(_k, v)| v.downcast_ref::<TodoAppError>().cloned())
        .flatten()
        .collect();
    println!("Errors: {errors:#?}");

    cfg_if! {
      if #[cfg(feature="ssr")]{
        let response = use_context::<ResponseOptions>(cx);
        if let Some(response) = response{
          response.set_status(StatusCode::from_u16(errors[0].code().unwrap().to_string().parse().unwrap()).unwrap());
        }
      }
    }

    view! {cx,
    <h1>"Errors"</h1>
    <For
        // a function that returns the items we're iterating over; a signal is fine
        each= move || {errors.clone().into_iter().enumerate()}
        // a unique key for each item as a reference
        key=|(index, _error)| index.clone()
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
