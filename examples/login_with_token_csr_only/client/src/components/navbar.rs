use crate::Page;
use leptos::*;
use leptos_router::*;

#[component]
pub fn NavBar<F>(
    cx: Scope,
    logged_in: Signal<bool>,
    on_logout: F,
) -> impl IntoView
where
    F: Fn() + 'static + Clone,
{
    view! { cx,
        <nav>
            <Show
                when=move || logged_in.get()
                fallback=|cx| {
                    view! { cx,
                        <A href=Page::Login.path()>"Login"</A>
                        " | "
                        <A href=Page::Register.path()>"Register"</A>
                    }
                }
            >
                <a
                    href="#"
                    on:click={
                        let on_logout = on_logout.clone();
                        move |_| on_logout()
                    }
                >
                    "Logout"
                </a>
            </Show>
        </nav>
    }
}
