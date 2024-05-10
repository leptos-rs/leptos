use leptos::prelude::*;
use timer::TimerDemo;

pub fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(TimerDemo)
}
