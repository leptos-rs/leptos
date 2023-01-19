use leptos::Errors;
use leptos::{view, For, ForProps, HydrationKey, IntoView, RwSignal, Scope, View};
use std::error::Error;

pub fn error_template(cx: Scope, errors: RwSignal<Errors>) -> View {
    println!("Errors: {:#?}", errors());
    view! {cx,
    <h1>"Errors"</h1>
    <For
        // a function that returns the items we're iterating over; a signal is fine
        each= move || {errors.get().0.into_iter()}
        // a unique key for each item as a reference
        key=|error| error.0.clone()
        // renders each item to a view
        view= move |error| {
            println!("SOURCE: {:#?}", error.1.source());
            // let source: String = error.1.source().unwrap().to_string();
            let error_string = error.1.to_string();
          view! {
            cx,
            <p>"Error: " {error_string}</p>
            // <p>"Location: " {source}</p>
          }
        }
      />
    }
    .into_view(cx)
}
