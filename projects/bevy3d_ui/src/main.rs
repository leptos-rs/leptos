mod demos;
mod routes;
use leptos::prelude::*;
use routes::RootPage;

pub fn main() {
    // Bevy will output a lot of debug info to the console when this is enabled.
    //_ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <RootPage/> })
}
