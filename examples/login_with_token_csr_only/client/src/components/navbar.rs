use crate::Page;
use leptos::prelude::*;
use leptos_router::*;

#[component]
pub fn NavBar(
    logged_in: Signal<bool>,
    #[prop(into)] on_logout: Callback<()>,
) -> impl IntoView {
    view! {
        <nav>
            <Show
                when=move || logged_in.get()
                fallback=|| {
                    view! {
                        <A href=Page::Login.path()>"Login"</A>
                        " | "
                        <A href=Page::Register.path()>"Register"</A>
                    }
                }
            >
                <a
                    href="#"
                    on:click=move |_| on_logout.call(())
                >
                    "Logout"
                </a>
            </Show>
        </nav>
    }
}
