use leptos::prelude::*;
use store_slotmap::App;

pub fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App)
}
