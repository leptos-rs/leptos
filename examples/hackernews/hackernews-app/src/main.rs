use hackernews_app::*;
use leptos::*;

pub fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    mount_to_body(|cx| {
        view! { cx, <App/> }
    });
}
