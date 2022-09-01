use counter_hydrate::simple_counter;
use leptos::*;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

pub fn main() {
    console_log::init_with_level(log::Level::Debug);
    hydrate(
        document()
            .get_element_by_id("hydrated")
            .unwrap()
            .unchecked_into(),
        simple_counter,
    );
    mount(
        document()
            .get_element_by_id("mounted")
            .unwrap()
            .unchecked_into(),
        simple_counter,
    );
}
