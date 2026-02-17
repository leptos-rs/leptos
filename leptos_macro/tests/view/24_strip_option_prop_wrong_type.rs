use leptos::prelude::*;

// Wrong type passed for a `#[prop(strip_option)]` prop.
// The inner type is `u8`, but we pass `"not_a_u8"` (`&str`).

#[component]
fn StripOptionInvalidType() -> impl IntoView {
    view! {
        <div>
            <Inner required=42 val="not_a_u8"/>
        </div>
    }
}

#[component]
fn Inner(
    required: i32,
    #[prop(strip_option)] val: Option<u8>,
) -> impl IntoView {
    let _ = required;
    let _ = val;
    ()
}

fn main() {}
