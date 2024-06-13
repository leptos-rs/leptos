use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::*;
mod api;
pub mod error_template;
#[cfg(feature = "ssr")]
pub mod fallback;
mod routes;
use routes::{nav::*, stories::*, story::*, users::*};

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/hackernews.css"/>
        <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
        <Meta name="description" content="Leptos implementation of a HackerNews demo."/>
        <Example/>
        <Router>
            <Nav />
            <main>
                <Routes>
                    <Route path="users/:id" view=User ssr=SsrMode::InOrder/>
                    <Route path="stories/:id" view=Story ssr=SsrMode::InOrder/>
                    <Route path=":stories?" view=Stories ssr=SsrMode::InOrder/>
                </Routes>
            </main>
        </Router>
    }
}

use leptos::prelude::*;

#[island]
pub fn CommonIsland() -> impl IntoView {
    let val = RwSignal::new(0);
    view! {
        <div>
            {move || format!("CommonIsland value is {}", val.get())}
            <button on:click=move|_| val.update(|x| {*x += 1})>Click</button>
        </div>

    }
}

#[island]
pub fn OuterWorking(children: Children) -> impl IntoView {
    let val = RwSignal::new(0);
    view! {
        <>
            <div>
                {move || format!("outer value is {}", val.get())}
                <button on:click=move|_| val.update(|x| {*x += 1})>Click</button>
            </div>
            {children()}
        </>

    }
}

#[component]
pub fn Example() -> impl IntoView {
    view! {
        <OuterFailing/>

        <OuterWorking>
            <CommonIsland/>
        </OuterWorking>

        <CommonIsland/>
    }
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();
    leptos::leptos_dom::HydrationCtx::stop_hydrating();
}
