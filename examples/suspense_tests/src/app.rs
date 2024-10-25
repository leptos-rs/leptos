use crate::instrumented::InstrumentedRoutes;
use leptos::prelude::*;
use leptos_router::{
    components::{Outlet, ParentRoute, Redirect, Route, Router, Routes, A},
    SsrMode, StaticSegment,
};

const WAIT_ONE_SECOND: u64 = 1;
const WAIT_TWO_SECONDS: u64 = 2;

#[server]
async fn first_wait_fn(seconds: u64) -> Result<(), ServerFnError> {
    tokio::time::sleep(tokio::time::Duration::from_secs(seconds)).await;

    Ok(())
}

#[server]
async fn second_wait_fn(seconds: u64) -> Result<(), ServerFnError> {
    tokio::time::sleep(tokio::time::Duration::from_secs(seconds)).await;

    Ok(())
}

#[component]
pub fn App() -> impl IntoView {
    let style = r"
        nav {
            display: flex;
            width: 100%;
            justify-content: space-around;
        }

        [aria-current] {
            font-weight: bold;
        }
    ";
    view! {
        <style>{style}</style>
        <Router>
            <nav>
                <A href="/out-of-order">"Out-of-Order"</A>
                <A href="/in-order">"In-Order"</A>
                <A href="/async">"Async"</A>
                <A href="/instrumented/">"Instrumented"</A>
            </nav>
            <main>
                <Routes fallback=|| "Page not found.">
                    <Route
                        path=StaticSegment("")
                        view=|| view! { <Redirect path="/out-of-order"/> }
                    />
                    // out-of-order
                    <ParentRoute
                        path=StaticSegment("out-of-order")
                        view=|| {
                            view! {
                                <SecondaryNav/>
                                <h1>"Out-of-Order"</h1>
                                <Outlet/>
                            }
                        }
                    >

                        <Route path=StaticSegment("") view=Nested/>
                        <Route path=StaticSegment("inside") view=NestedResourceInside/>
                        <Route path=StaticSegment("single") view=Single/>
                        <Route path=StaticSegment("parallel") view=Parallel/>
                        <Route path=StaticSegment("inside-component") view=InsideComponent/>
                        <Route path=StaticSegment("local") view=LocalResource/>
                        <Route path=StaticSegment("none") view=None/>
                    </ParentRoute>
                    // in-order
                    <ParentRoute
                        path=StaticSegment("in-order")
                        ssr=SsrMode::InOrder
                        view=|| {
                            view! {
                                <SecondaryNav/>
                                <h1>"In-Order"</h1>
                                <Outlet/>
                            }
                        }
                    >

                        <Route path=StaticSegment("") view=Nested/>
                        <Route path=StaticSegment("inside") view=NestedResourceInside/>
                        <Route path=StaticSegment("single") view=Single/>
                        <Route path=StaticSegment("parallel") view=Parallel/>
                        <Route path=StaticSegment("inside-component") view=InsideComponent/>
                        <Route path=StaticSegment("local") view=LocalResource/>
                        <Route path=StaticSegment("none") view=None/>
                    </ParentRoute>
                    // async
                    <ParentRoute
                        path=StaticSegment("async")
                        ssr=SsrMode::Async
                        view=|| {
                            view! {
                                <SecondaryNav/>
                                <h1>"Async"</h1>
                                <Outlet/>
                            }
                        }
                    >

                        <Route path=StaticSegment("") view=Nested/>
                        <Route path=StaticSegment("inside") view=NestedResourceInside/>
                        <Route path=StaticSegment("single") view=Single/>
                        <Route path=StaticSegment("parallel") view=Parallel/>
                        <Route path=StaticSegment("inside-component") view=InsideComponent/>
                        <Route path=StaticSegment("local") view=LocalResource/>
                        <Route path=StaticSegment("none") view=None/>
                    </ParentRoute>
                    <InstrumentedRoutes/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn SecondaryNav() -> impl IntoView {
    view! {
        <nav>
            <A href="" exact=true>
                "Nested"
            </A>
            <A href="inside" exact=true>
                "Nested (resource created inside)"
            </A>
            <A href="single">"Single"</A>
            <A href="parallel">"Parallel"</A>
            <A href="inside-component">"Inside Component"</A>
            <A href="local">"Local Resource"</A>
            <A href="none">"No Resources"</A>
        </nav>
    }
}

#[component]
fn Nested() -> impl IntoView {
    let one_second = Resource::new(|| WAIT_ONE_SECOND, first_wait_fn);
    let two_second = Resource::new(|| WAIT_TWO_SECONDS, second_wait_fn);
    let (count, set_count) = signal(0);

    view! {
        <div>
            <Suspense fallback=|| {
                "Loading 1..."
            }>
                {move || {
                    one_second.map(|_| view! { <p id="loaded-1">"One Second: Loaded 1!"</p> })
                }}
                <Suspense fallback=|| {
                    "Loading 2..."
                }>
                    {move || {
                        two_second
                            .map(|_| {
                                view! {
                                    <p id="loaded-2">"Two Second: Loaded 2!"</p>
                                    <button on:click=move |_| {
                                        set_count.update(|n| *n += 1)
                                    }>{count}</button>
                                }
                            })
                    }}

                </Suspense>
            </Suspense>
        </div>
    }
}

#[component]
fn NestedResourceInside() -> impl IntoView {
    let one_second = Resource::new(|| WAIT_ONE_SECOND, first_wait_fn);
    let (count, set_count) = signal(0);

    view! {
        <div>
            <Suspense fallback=|| {
                "Loading 1..."
            }>
                {Suspend::new(async move {
                    _ = one_second.await;
                    let two_second = Resource::new(
                        || (),
                        move |_| async move { second_wait_fn(WAIT_TWO_SECONDS).await },
                    );
                    view! {
                        <p id="loaded-1">"One Second: Loaded 1!"</p>
                        <Suspense fallback=|| "Loading 2...">
                            <span id="loaded-2">
                                "Loaded 2 (created inside first suspense)!: "
                                {Suspend::new(async move { format!("{:?}", two_second.await) })}
                            </span>
                            <button on:click=move |_| set_count.update(|n| *n += 1)>{count}</button>
                        </Suspense>
                    }
                })}

            </Suspense>
        </div>
    }
}

#[component]
fn Parallel() -> impl IntoView {
    let one_second = Resource::new(|| WAIT_ONE_SECOND, first_wait_fn);
    let two_second = Resource::new(|| WAIT_TWO_SECONDS, second_wait_fn);
    let (count, set_count) = signal(0);

    view! {
        <div>
            <Suspense fallback=|| {
                "Loading 1..."
            }>
                {move || {
                    one_second
                        .map(move |_| {
                            view! {
                                <p id="loaded-1">"One Second: Loaded 1!"</p>
                                <button on:click=move |_| {
                                    set_count.update(|n| *n += 1)
                                }>{count}</button>
                            }
                        })
                }}

            </Suspense>
            <Suspense fallback=|| {
                "Loading 2..."
            }>
                {move || {
                    two_second
                        .map(move |_| {
                            view! {
                                <p id="loaded-2">"Two Second: Loaded 2!"</p>
                                <button
                                    id="second-count"
                                    on:click=move |_| set_count.update(|n| *n += 1)
                                >
                                    {count}
                                </button>
                            }
                        })
                }}

            </Suspense>
        </div>
    }
}

#[component]
fn Single() -> impl IntoView {
    let one_second = Resource::new(|| WAIT_ONE_SECOND, first_wait_fn);
    let (count, set_count) = signal(0);

    view! {
        <div>
            <Suspense fallback=|| {
                "Loading 1..."
            }>
                {move || {
                    one_second.map(|_| view! { <p id="loaded-1">"One Second: Loaded 1!"</p> })
                }}

            </Suspense>
            <p id="following-message">"Children following Suspense should hydrate properly."</p>
            <div>
                <button on:click=move |_| set_count.update(|n| *n += 1)>{count}</button>
            </div>
        </div>
    }
}

#[component]
fn InsideComponent() -> impl IntoView {
    let (count, set_count) = signal(0);

    view! {
        <div>
            <p id="inside-message">"Suspense inside another component should work."</p>
            <InsideComponentChild/>
            <p id="following-message">"Children following Suspense should hydrate properly."</p>
            <div>
                <button on:click=move |_| set_count.update(|n| *n += 1)>{count}</button>
            </div>
        </div>
    }
}

#[component]
fn InsideComponentChild() -> impl IntoView {
    let one_second = Resource::new(|| WAIT_ONE_SECOND, first_wait_fn);
    view! {
        <Suspense fallback=|| {
            "Loading 1..."
        }>
            {move || {
                one_second.map(|_| view! { <p id="loaded-1">"One Second: Loaded 1!"</p> })
            }}

        </Suspense>
    }
}

#[component]
fn LocalResource() -> impl IntoView {
    let one_second = Resource::new(|| WAIT_ONE_SECOND, first_wait_fn);
    let local = LocalResource::new(|| first_wait_fn(WAIT_ONE_SECOND));
    let (count, set_count) = signal(0);

    view! {
        <div>
            <Suspense fallback=|| {
                "Loading 1..."
            }>
                {move || {
                    one_second.map(|_| view! { <p id="loaded-1">"One Second: Loaded 1!"</p> })
                }}
                {move || {
                    Suspend::new(async move {
                        let value = local.await;
                        view! { <p id="loaded-2">"One Second: Local Loaded " {value} "!"</p> }
                    })
                }}

            </Suspense>
            <p id="following-message">"Children following Suspense should hydrate properly."</p>
            <div>
                <button on:click=move |_| set_count.update(|n| *n += 1)>{count}</button>
            </div>
        </div>
    }
}

#[component]
fn None() -> impl IntoView {
    let (count, set_count) = signal(0);

    view! {
        <div>
            <Suspense fallback=|| "Loading 1...">
                <p id="inside-message">"Children inside Suspense should hydrate properly."</p>
                <button on:click=move |_| set_count.update(|n| *n += 1)>{count}</button>
            </Suspense>
            <p id="following-message">"Children following Suspense should hydrate properly."</p>
            <div>
                <button id="second-count" on:click=move |_| set_count.update(|n| *n += 1)>
                    {count}
                </button>
            </div>
        </div>
    }
}
