// Component with only optional props. Should compile without errors
// when invoked with no props.

use leptos::prelude::*;

#[component]
fn OnlyOptional() -> impl IntoView {
    view! {
        <div>
            <Inner/>
        </div>
    }
}

#[component]
fn Inner(
    #[prop(optional)] flag: bool,
    #[prop(default = 42)] count: i32,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    let _ = flag;
    let _ = count;
    children.map(|c| c())
}

fn main() {}
