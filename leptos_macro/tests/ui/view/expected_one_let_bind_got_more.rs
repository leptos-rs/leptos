use leptos::*;

#[component]
fn Component<C, IV>(children: C) -> impl IntoView
where
    C: Fn(String) -> IV,
    IV: IntoView,
{
    _ = children;
}

fn main() {
    view! {
        <Component
            let:item
            let:extra_item
        >
            <p>{item}</p>
        </Component>
    };
}
