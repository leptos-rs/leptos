use leptos::{view, Errors, For, IntoView, RwSignal, Scope, View};

// A basic function to display errors served by the error boundaries. Feel free to do more complicated things
// here than just displaying them
pub fn error_template(cx: Scope, errors: Option<RwSignal<Errors>>) -> View {
    let Some(errors) = errors else {
        panic!("No Errors found and we expected errors!");
    };

    view! {cx,
      <h1>"Errors"</h1>
      <For
          // a function that returns the items we're iterating over; a signal is fine
          each=errors
          // a unique key for each item as a reference
          key=|(key, _)| key.clone()
          // renders each item to a view
          view= move |cx, (_, error)| {
          let error_string = error.to_string();
            view! {
              cx,
              <p>"Error: " {error_string}</p>
            }
          }
      />
    }
    .into_view(cx)
}
