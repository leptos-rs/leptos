use counter_url_query::SimpleQueryCounter;
use leptos::prelude::*;
use leptos_router::components::Router;

pub fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(|| {
        view! {
            <Router>
                <SimpleQueryCounter/>
            </Router>
        }
    })
}
