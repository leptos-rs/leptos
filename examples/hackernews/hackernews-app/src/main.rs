use hackernews_app::*;
use leptos::*;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

pub fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    mount_to_body(|cx| {
        view! { cx,
            <div>
                <Router mode=BrowserIntegration {}><App/></Router>
            </div>
        }
    })
}
