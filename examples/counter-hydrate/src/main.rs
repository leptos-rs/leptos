use counter_hydrate::*;
use leptos::*;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

pub fn main() {
    console_log::init_with_level(log::Level::Debug);
    hydrate(body().unwrap(), |cx| view! { <Counters/> });
    /* mount(
        document()
            .get_element_by_id("mounted")
            .unwrap()
            .unchecked_into(),
        simple_counter,
    ); */
}
