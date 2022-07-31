use leptos::*;

use leptos_dom::wasm_bindgen::{JsCast, UnwrapThrowExt};

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

fn main() {
    mount_to_body(|cx| {
        view! { <Parent/> }
    })
}

type CounterHolder = Vec<(usize, (ReadSignal<i32>, WriteSignal<i32>))>;

struct CounterUpdater {
    set_counters: WriteSignal<CounterHolder>,
}

#[component]
fn Parent(cx: Scope) -> web_sys::Element {
    let (value, _) = cx.create_signal(0);

    view! {
        <div>
            "regular"
            <h1>"Children should appear below"</h1>
            {|| create_text_node(&value.get().to_string())}
            <HasChildren>
                <span>"Child A"</span>
                <span>"Child B"</span>
                <span>"Child C"</span>
            </HasChildren>
        </div>
    }
}

#[component]
fn HasChildren(cx: Scope, children: Vec<Element>) -> web_sys::Element {
    view! {
        <div>
            <h2>"I have children:"</h2>
            <ul>
            {
                children.into_iter().map(|child| {
                    debug_warn!("child is {:?}", child.outer_html());
                    view! { <li>{child}</li> }
                }).collect::<Vec<_>>()
            }
            </ul>
        </div>
    }
}
