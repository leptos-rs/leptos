use crate::errors::AppError;
use leptos::{logging::log, prelude::*};
#[cfg(feature = "ssr")]
use leptos_axum::ResponseOptions;

// A basic function to display errors served by the error boundaries.
// Feel free to do more complicated things here than just displaying them.
#[component]
pub fn ErrorTemplate(#[prop(into)] errors: Signal<Errors>) -> impl IntoView {
    // Get Errors from Signal
    // Downcast lets us take a type that implements `std::error::Error`
    let errors = Memo::new(move |_| {
        errors
            .get_untracked()
            .into_iter()
            .filter_map(|(_, v)| v.downcast_ref::<AppError>().cloned())
            .collect::<Vec<_>>()
    });
    log!("Errors: {:#?}", &*errors.read_untracked());

    // Only the response code for the first error is actually sent from the server
    // this may be customized by the specific application
    #[cfg(feature = "ssr")]
    {
        let response = use_context::<ResponseOptions>();
        if let Some(response) = response {
            response.set_status(errors.read_untracked()[0].status_code());
        }
    }

    view! {
        <h1>{move || {
            if errors.read().len() > 1 {
                "Errors"
            } else {
                "Error"
            }}}
        </h1>
        {move || {
            errors.get()
                .into_iter()
                .map(|error| {
                    let error_string = error.to_string();
                    let error_code= error.status_code();
                    view! {
                        <h2>{error_code.to_string()}</h2>
                        <p>"Error: " {error_string}</p>
                    }
                })
                .collect_view()
        }}
    }
}
