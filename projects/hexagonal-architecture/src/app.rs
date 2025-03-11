use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};

#[cfg(feature = "ssr")]
use super::{server_types::HandlerStructAlias, traits::HandlerTrait};
use crate::ui_types::*;
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/leptos-hexagonal-design.css"/>

        // sets the document title
        <Title text="Welcome to Leptos"/>

        // content for this welcome page
        <Router>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=HomePage/>
                </Routes>
            </main>
        </Router>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    let server_fn_1 = ServerAction::<ServerFunction1>::new();
    let server_fn_2 = ServerAction::<ServerFunction2>::new();
    let server_fn_3 = ServerAction::<ServerFunction3>::new();
    Effect::new(move |_| {
        server_fn_1.dispatch(ServerFunction1 {});
        server_fn_2.dispatch(ServerFunction2 {});
        server_fn_3.dispatch(ServerFunction3 {});
    });
}

#[server]
#[middleware(crate::middleware::SubDomain1Layer)]
pub async fn server_function_1() -> Result<UiMappingFromDomainData, ServerFnError> {
    Ok(expect_context::<HandlerStructAlias>()
        .server_fn_1()
        .await?
        .into())
}
#[server]
pub async fn server_function_2() -> Result<UiMappingFromDomainData2, ServerFnError> {
    Ok(expect_context::<HandlerStructAlias>()
        .server_fn_2()
        .await?
        .into())
}
#[server]
pub async fn server_function_3() -> Result<UiMappingFromDomainData3, ServerFnError> {
    Ok(expect_context::<HandlerStructAlias>()
        .server_fn_3()
        .await?
        .into())
}
