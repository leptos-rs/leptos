use leptos::prelude::*;
use leptos_router::{
    components::{Outlet, ParentRoute, Route},
    path, Lazy, LazyRoute, MatchNestedRoutes,
};

// https://github.com/leptos-rs/leptos/issues/4971

#[component]
pub fn Routes4971() -> impl MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=path!("4971") view=Layout4971>
            <Route path=path!("") view={Lazy::<Home4971>::new()}/>
            <Route path=path!("slow") view={Lazy::<Slow4971>::new()}/>
            <Route path=path!("other") view={Lazy::<Other4971>::new()}/>
        </ParentRoute>
    }
    .into_inner()
}

#[component]
fn Layout4971() -> impl IntoView {
    view! {
        <nav>
            <a href="/4971/slow">"Slow 4971"</a>
            <a href="/4971/other">"Other 4971"</a>
        </nav>
        <Outlet/>
    }
}

/// Route whose data is read by its view after it has been mounted.
struct Home4971 {
    count: RwSignal<i32>,
}

impl LazyRoute for Home4971 {
    fn data() -> Self {
        Self {
            count: RwSignal::new(0),
        }
    }

    async fn view(this: Self) -> AnyView {
        let Home4971 { count } = this;
        view! {
            <p id="result">{move || count.get()}</p>
            <button id="increment-4971" on:click=move |_| count.set(count.get() + 1)>
                "+1"
            </button>
        }
        .into_any()
    }
}

/// Routes that take a while to load, so that the previous route stays on screen meanwhile.
struct Slow4971;

impl LazyRoute for Slow4971 {
    fn data() -> Self {
        Self
    }

    async fn preload() {
        _ = delay_4971().await;
    }

    async fn view(_this: Self) -> AnyView {
        view! { <p id="page-4971">"Slow"</p> }.into_any()
    }
}

struct Other4971;

impl LazyRoute for Other4971 {
    fn data() -> Self {
        Self
    }

    async fn preload() {
        _ = delay_4971().await;
    }

    async fn view(_this: Self) -> AnyView {
        view! { <p id="page-4971">"Other"</p> }.into_any()
    }
}

#[server]
async fn delay_4971() -> Result<(), ServerFnError> {
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
    Ok(())
}
