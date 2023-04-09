use leptos::*;
use leptos_meta::*;
use leptos_router::*;

mod routes;
use routes::about::*;
use routes::blog::*;
use routes::error::*;
use routes::home::*;
use routes::post::*;

// TODO: handle responsive

#[allow(non_snake_case)]
pub fn App(cx: Scope) -> Element {
    provide_context(cx, MetaContext::default());
    view! {
        cx,
        <div id="root">
            <Router>
                <main class="container">
                    <Routes>
                        <Route path="" element=move |_cx| view! { cx, <Home/> } />
                        <Route path="blog" element=move |_cx| view! { cx, <Blog/> } />
                        <Route path="blog/:id" element=move |_cx| view! {cx, <Post /> }/>
                        <Route path="about" element=move |_cx| view! { cx, <About/> } />

                        <Route path="*" element=move |_cx| view! { cx, <PageNotFound/> } />
                    </Routes>
                </main>
            </Router>
        </div>
    }
}
