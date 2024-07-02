use leptos::prelude::*;

// A basic function to display errors served by the error boundaries. Feel free to do more complicated things
// here than just displaying them
#[component]
pub fn ErrorTemplate(
    #[prop(optional)] outside_errors: Option<Errors>,
    #[prop(optional, into)] errors: Option<RwSignal<Errors>>,
) -> impl IntoView {
    let errors = match outside_errors {
        Some(e) => RwSignal::new(e),
        None => match errors {
            Some(e) => e,
            None => panic!("No Errors found and we expected errors!"),
        },
    };

    // Get Errors from Signal
    // Downcast lets us take a type that implements `std::error::Error`
    let errors =
        move || errors.get().into_iter().map(|(_, v)| v).collect::<Vec<_>>();

    // Only the response code for the first error is actually sent from the server
    // this may be customized by the specific application
    /*#[cfg(feature = "ssr")]
    {
        let response = use_context::<ResponseOptions>();
        if let Some(response) = response {
            response.set_status(errors[0].status_code());
        }
    }*/

    view! {
        <h1>"Errors"</h1>
        {move || {
            errors()
                .into_iter()
                .map(|error| {
                    view! { <p>"Error: " {error.to_string()}</p> }
                })
                .collect::<Vec<_>>()
        }}
    }
}
