use leptos::prelude::*;
use leptos_router::LazyRoute;
#[allow(unused_imports)]
use leptos_router::{
    components::Route, path, Lazy, MatchNestedRoutes, NavigateOptions,
};

#[component]
pub fn Routes4285() -> impl MatchNestedRoutes + Clone {
    view! {
        <Route path=path!("4285") view={Lazy::<Issue4285>::new()}/>
    }
    .into_inner()
}

struct Issue4285 {
    data: Resource<Result<i32, ServerFnError>>,
}

impl LazyRoute for Issue4285 {
    fn data() -> Self {
        Self {
            data: Resource::new(|| (), |_| slow_call()),
        }
    }

    async fn view(this: Self) -> AnyView {
        let Issue4285 { data } = this;
        view! {
            <Suspense>
                {move || {
                    Suspend::new(async move {
                        let data = data.await;
                        view! {
                            <p id="result">{data}</p>
                        }
                    })
                }}
            </Suspense>
        }
        .into_any()
    }
}

#[server]
async fn slow_call() -> Result<i32, ServerFnError> {
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    Ok(42)
}
