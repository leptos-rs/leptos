use crate::{log::SimpleLogger, issue_4088::Routes4088, pr_4015::Routes4015, pr_4091::Routes4091};
use leptos::prelude::*;
use leptos_meta::{MetaTags, *};
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone()/>
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
    provide_meta_context();
    let logger = SimpleLogger::default();
    provide_context(logger.clone());
    let fallback = || view! { "Page not found." }.into_view();
    view! {
        <Stylesheet id="leptos" href="/pkg/regression.css"/>
        <Router>
            <main>
                <Routes fallback>
                    <Route path=path!("") view=HomePage/>
                    <Route path=path!("README") view=Readme/>
                    <Routes4091/>
                    <Routes4015/>
                    <Routes4088/>
                </Routes>
            </main>
        </Router>
        <footer>
            <section id="log">{move || logger.render() }</section>
        </footer>
    }
}

#[server]
async fn server_call() -> Result<(), ServerFnError> {
    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    Ok(())
}

#[component]
fn HomePage() -> impl IntoView {
    view! {
        <Title text="Regression Tests"/>
        <h1>"Listing of regression tests"</h1>
        <p><a href="/README">(What is this)</a></p>
        <nav>
            <ul>
                <li><a href="/4091/">"4091"</a></li>
                <li><a href="/4015/">"4015"</a></li>
                <li><a href="/4088/">"4088"</a></li>
            </ul>
        </nav>
    }
}

static EXAMPLE: &'static str = "\
use leptos::prelude::*;
use crate::log::SimpleLogger;
let logger = expect_context::<SimpleLogger>();
logger.log(\"Hello world!\");";

#[component]
fn Readme() -> impl IntoView {
    view! {
        <h1>"About regression example"</h1>
        <p>"
            This is a collection of components containing the minimum reproducible example that
            should work without issues, but have possibly failed some time in the past in the form
            of a regression.  The components are self contained in their respective modules and
            should be accompanied by an end-to-end test suite written in Gherkin, to allow a human
            user to also reproduce and validate the expected behavior from the written instructions.
        "</p>
        // TODO probably establish some naming conventions here?
        <p>"
            A logger output pane is provided on the side, which may be invoked within a component
            in this example like so:
        "</p>
        <blockquote><pre><code>{EXAMPLE}</code></pre></blockquote>
        <p>"
            This "<a href="#" on:click=|_| {
                use crate::log::SimpleLogger;
                let logger = expect_context::<SimpleLogger>();
                logger.log("Hello world!");
            }>"example link"</a>" is hooked with the above, so accessing that should result in that
            message printed, while this "<a href="#" on:click=|_| {
                use crate::log::SimpleLogger;
                let logger = expect_context::<SimpleLogger>();
                logger.log("Something else.");
            }>"other link"</a>" will log something else. "<a href="/">"Return to listing"</a>".
        "</p>
    }
}
