use counter_without_macros as counter;
use leptos::*;

pub fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    mount_to_body(|cx| {
        counter::view(
            cx,
            counter::Props {
                initial_value: 0,
                step: 1,
            },
        )
    })
}
