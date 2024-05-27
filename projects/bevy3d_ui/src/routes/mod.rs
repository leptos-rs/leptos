pub mod demo1;
use demo1::Demo1;
use leptos::*;
use leptos_meta::{provide_meta_context, Meta, Stylesheet, Title};
use leptos_router::*;

#[component]
pub fn RootPage() -> impl IntoView {
    provide_meta_context();

    view! {
        <Meta name="charset" content="UTF-8"/>
        <Meta name="description" content="Leptonic CSR template"/>
        <Meta name="viewport" content="width=device-width, initial-scale=1.0"/>
        <Meta name="theme-color" content="#e66956"/>
        <Stylesheet href="https://fonts.googleapis.com/css?family=Roboto&display=swap"/>
        <Title text="Leptos Bevy3D Example"/>
        <Router>
            <Routes>
                <Route path="" view=|| view! { <Demo1/> }/>
            </Routes>
        </Router>
    }
}
