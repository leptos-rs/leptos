use leptos::*;

#[component]
fn Outer(children: ChildrenFn) -> impl IntoView {
    _ = children;
}

#[component]
fn Inner(children: ChildrenFn) -> impl IntoView {
    _ = children;
}

#[component]
fn Inmost(name: String) -> impl IntoView {
    _ = name;
}

fn main() {
    let name = "Alice".to_string();

    view! {
        <Outer>
            <Inner>
                <Inmost name=name.clone()/>
            </Inner>
        </Outer>
    };
}
