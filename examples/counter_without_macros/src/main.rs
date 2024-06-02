use counter_without_macros::counter;

/// Show the counter
pub fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(|| counter(0, 1))
}
