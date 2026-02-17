use leptos::prelude::*;

#[component]
fn BareAndWrapped() -> impl IntoView {
    view! {
        <div>
            <Inner bare=|| true wrapped=vec![|| true]/>
        </div>
    }
}

#[component]
fn Inner<F>(bare: F, wrapped: Vec<F>) -> impl IntoView
where
    F: Fn() -> bool,
{
    let _ = bare();
    for f in &wrapped {
        let _ = f();
    }
    ()
}

fn main() {}
