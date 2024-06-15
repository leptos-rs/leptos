use leptos::{view, Errors, For, IntoView, RwSignal, SignalGet, View};

// A basic function to display errors served by the error boundaries. Feel free to do more complicated things
// here than just displaying them
pub fn error_template(errors: RwSignal<Errors>) -> View {
    view! {
      <h1>"Errors"</h1>
      <For
          // a function that returns the items we're iterating over; a signal is fine
          each=move || errors.get()
          // a unique key for each item as a reference
          key=|(key, _)| key.clone()
          // renders each item to a view
          children= move | (_, error)| {
          let error_string = error.to_string();
            view! {
              <p>"Error: " {error_string}</p>
            }
          }
      />
    }
    .into_view()
}
