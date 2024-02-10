use leptos::*;

#[component]
fn Root(outer: Outer) -> impl IntoView {
    _ = outer;
}

#[slot]
struct Outer {
    children: ChildrenFn,
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
        <Root>
            <Outer slot>
                <Inner>
                    <Inmost name=name.clone() />
                </Inner>
            </Outer>
        </Root>
    };
}
