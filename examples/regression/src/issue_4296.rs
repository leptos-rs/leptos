use leptos::prelude::*;
#[allow(unused_imports)]
use leptos_router::{
    components::Route, path, Lazy, MatchNestedRoutes, NavigateOptions,
};
use leptos_router::{hooks::use_query_map, LazyRoute};

#[component]
pub fn Routes4296() -> impl MatchNestedRoutes + Clone {
    view! {
        <Route path=path!("4296") view={Lazy::<Issue4296>::new()}/>
    }
    .into_inner()
}

struct Issue4296 {
    query: Signal<Option<String>>,
}

impl LazyRoute for Issue4296 {
    fn data() -> Self {
        let query = use_query_map();
        let query = Signal::derive(move || query.read().get("q"));
        Self { query }
    }

    async fn view(this: Self) -> AnyView {
        let Issue4296 { query } = this;
        view! {
            <a href="?q=abc">"abc"</a>
            <a href="?q=def">"def"</a>
            <p id="result">{move || format!("{:?}", query.get())}</p>
        }
        .into_any()
    }
}
