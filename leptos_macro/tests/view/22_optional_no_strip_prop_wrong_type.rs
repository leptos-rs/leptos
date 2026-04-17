use leptos::prelude::*;

// Wrong type passed for an `#[prop(optional_no_strip)]` prop.
// The prop type is `Option<String>`, but we pass a bare `"hello"`
// (`&str`), which is not `Option<String>`.

#[component]
fn OptionalNoStripInvalidType() -> impl IntoView {
    view! {
        <div>
            <Inner required=42 val="hello"/>
        </div>
    }
}

#[component]
fn Inner(
    required: i32,
    #[prop(optional_no_strip)] val: Option<String>,
) -> impl IntoView {
    let _ = required;
    let _ = val;
    ()
}

fn main() {}
