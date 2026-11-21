#![cfg(target_arch = "wasm32")]

use leptos::{either::Either, mount::mount_to, prelude::*};
use leptos_meta::{Title, provide_meta_context};
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::HtmlElement;

wasm_bindgen_test_configure!(run_in_browser);

#[component]
fn FirstRoute() -> impl IntoView {
    view! {
        <Title text="First Route"/>
        <p>"First"</p>
    }
}

#[component]
fn SecondRoute(title: RwSignal<String>) -> impl IntoView {
    view! {
        <Title text=move || title.get()/>
        <p>"Second"</p>
    }
}

async fn settle() {
    for _ in 0..4 {
        leptos::task::tick().await;
    }
}

#[wasm_bindgen_test(async)]
async fn title_effect_survives_unmounting_first_title_scope() {
    let document = web_sys::window().unwrap_throw().document().unwrap_throw();
    let root = document
        .create_element("div")
        .unwrap_throw()
        .dyn_into::<HtmlElement>()
        .unwrap_throw();
    document
        .body()
        .unwrap_throw()
        .append_child(&root)
        .unwrap_throw();

    let show_second = RwSignal::new(false);
    let second_title = RwSignal::new("Second Route 1".to_string());

    let mounted = mount_to(root.clone(), move || {
        provide_meta_context();
        move || {
            if show_second.get() {
                Either::Right(view! { <SecondRoute title=second_title/> })
            } else {
                Either::Left(view! { <FirstRoute/> })
            }
        }
    });

    settle().await;
    assert_eq!(document.title(), "First Route");

    // Unmounting `FirstRoute` drops the child scope under which `<Title/>` first spawned
    // `TitleContext`'s singleton `RenderEffect`.
    show_second.set(true);
    settle().await;
    assert_eq!(document.title(), "Second Route 1");

    second_title.set("Second Route 2".to_string());
    settle().await;
    assert_eq!(document.title(), "Second Route 2");

    drop(mounted);
    root.remove();
}
