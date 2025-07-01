use leptos::{context::Provider, prelude::*};
use leptos_router::{
    components::{ParentRoute, Route, A},
    nested_router::Outlet,
    path,
};

// FIXME This should be a set rather than a naive vec for push and pop, as
// it may be possible for unexpected token be popped/pushed on multi-level
// navigation.  For basic naive tests it should be Fine(TM).
#[derive(Clone)]
struct Expectations(Vec<&'static str>);

#[component]
pub fn Routes4091() -> impl leptos_router::MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=path!("4091") view=Container>
            <Route path=path!("") view=Root/>
            <Route path=path!("test1") view=Test1/>
        </ParentRoute>
    }
    .into_inner()
}

#[component]
fn Container() -> impl IntoView {
    let rw_signal = RwSignal::new(Expectations(Vec::new()));
    provide_context(rw_signal);

    view! {
        <nav id="nav">
            <ul>
                <li><A href="/">"Home"</A></li>
                <li><A href="./">"4091 Home"</A></li>
                <li><A href="test1">"test1"</A></li>
            </ul>
        </nav>
        <div id="result">{move || {
            rw_signal.with(|ex| ex.0.iter().fold(String::new(), |a, b| a + b + " "))
        }}</div>
        <Provider value=rw_signal>
            <Outlet/>
        </Provider>
    }
}

#[component]
fn Root() -> impl IntoView {
    view! {
        <div>"This is Root"</div>
    }
}

#[component]
fn Test1() -> impl IntoView {
    let signal = expect_context::<RwSignal<Expectations>>();

    on_cleanup(move || {
        signal.update(|ex| {
            ex.0.pop();
        });
    });

    view! {
        {move || signal.update(|ex| ex.0.push("Test1"))}
        <div>"This is Test1"</div>
    }
}
