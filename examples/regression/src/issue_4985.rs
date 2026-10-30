use leptos::prelude::*;
use leptos_router::{
    components::{Outlet, ParentRoute, Route},
    path, Lazy, LazyRoute, MatchNestedRoutes,
};

// https://github.com/leptos-rs/leptos/issues/4985

#[component]
pub fn Routes4985() -> impl MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=path!("4985") view=Page4985>
            <ParentRoute path=path!("suspense") view=SuspenseOutlet>
                <Route path=path!("") view={Lazy::<Issue4985>::new()}/>
            </ParentRoute>
            <ParentRoute path=path!("plain") view=Outlet>
                <Route path=path!("") view={Lazy::<Issue4985>::new()}/>
            </ParentRoute>
        </ParentRoute>
    }
    .into_inner()
}

#[component]
fn Page4985() -> impl IntoView {
    view! {
        <nav>
            <a href="/4985/suspense">"Suspense"</a>
            <a href="/4985/plain">"Plain"</a>
        </nav>
        <Outlet/>
    }
}

#[component]
fn SuspenseOutlet() -> impl IntoView {
    view! {
        <Suspense>
            <Outlet/>
        </Suspense>
    }
}

pub const MARKER: &str = "ISSUE_4985_MARKER";

#[derive(Clone)]
struct Issue4985 {
    data: Resource<String>,
}

impl LazyRoute for Issue4985 {
    fn data() -> Self {
        Self {
            data: Resource::new(|| (), |_| async { MARKER.to_string() }),
        }
    }

    async fn view(this: Self) -> AnyView {
        let Issue4985 { data } = this;
        view! {
            <Suspense>
                <p id="result">
                    {move || Suspend::new(async move { data.await.len() })}
                </p>
            </Suspense>
        }
        .into_any()
    }
}
