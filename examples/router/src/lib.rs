mod api;

use leptos::*;
use leptos_router::*;

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    console_error_panic_hook::set_once();

    view! {cx,
        <Router>
            <nav>
                <a href="/">"home"</a> " | "
                <a href="/a">"a"</a> " | "
                <a href="/b">"b"</a> " | "
                <a href="/some-page">"some page"</a>
            </nav>
            <Routes>
                <Route path="/" view=|cx| {
                    log!("/ rendering on {:?}", cx.id());

                    on_cleanup(cx, || {
                        log!("home: cleaning up");
                    });

                    view!{cx,
                        <main><Outlet/></main>
                    }
                }>
                    <Route path="/a" view=|cx| {
                        log!("/a rendering on {:?}", cx.id());

                        on_cleanup(cx, || {
                            log!("A: cleaning up");
                        });

                        view!{cx,
                            <p>"I am A"</p>
                        }
                    } />
                    <Route path="/b" view=|cx| {
                        log!("/b rendering on {:?}", cx.id());
                        on_cleanup(cx, || {
                            log!("B: cleaning up");
                        });

                        view!{cx,
                            <p>"I am B"</p>
                        }
                    } />
                    <Route path="" view=|cx| {
                        log!("/(home) rendering on {:?}", cx.ancestry());

                        on_cleanup(cx, || {
                            log!("home 2: cleaning up");
                        });

                        view!{cx,
                            <p>"I am home"</p>
                        }
                    } />
                </Route>
                <Route path="/some-page" view=|cx| {
                    on_cleanup(cx, || {
                        log!("some-page: cleaning up");
                    });

                    view!{cx,
                        <p>"I am some page"</p>
                    }
                }/>

            </Routes>
        </Router>
    }
}
