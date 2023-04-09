use blog::App;
use leptos::mount_to_body;

fn main() {
    _ = console_log::init_with_level(log::Level::Debug).expect("[Init log level] Error");
    console_error_panic_hook::set_once();
    mount_to_body(App);
}
