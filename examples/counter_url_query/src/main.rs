use counter_url_query::SimpleQueryCounter;
use leptos::*;
use leptos_router::*;

pub fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    mount_to_body(|| {
        view! {
            <Router>
                <Routes>
                    <Route path="" view=SimpleQueryCounter />
                </Routes>
            </Router>
        }
    })
}
