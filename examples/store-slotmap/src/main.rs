use leptos::prelude::*;
use nested_stores::App;

pub fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App)
}
