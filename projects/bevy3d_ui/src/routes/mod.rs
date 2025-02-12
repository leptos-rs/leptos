pub mod demo1;
use demo1::Demo1;
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet};
use leptos_router::components::*;
use leptos_router::StaticSegment;
#[component]
pub fn RootPage() -> impl IntoView {
    provide_meta_context();

    view! {
        <meta name="charset" content="UTF-8"/>
        <meta name="description" content="Leptonic CSR template"/>
        <meta name="viewport" content="width=device-width, initial-scale=1.0"/>
        <meta name="theme-color" content="#e66956"/>
        <Title text="Leptos Bevy3D Example"/>
        <Stylesheet href="https://fonts.googleapis.com/css?family=Roboto&display=swap"/>
        <MetaTags/>
        <Router>
            <Routes fallback=move || "Not found.">
                <Route path=StaticSegment("") view=Demo1 />
            </Routes>
        </Router>
    }
}
