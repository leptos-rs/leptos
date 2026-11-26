#![cfg(all(target_arch = "wasm32", feature = "hydrate"))]

use leptos::{
    mount::{hydrate_from, hydrate_from_async},
    prelude::*,
};
use std::{
    fmt,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::HtmlElement;

wasm_bindgen_test_configure!(run_in_browser);

#[derive(Clone, Debug)]
struct TestError(&'static str);

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for TestError {}

fn init_hydration_globals() {
    let window = web_sys::window().unwrap_throw();
    js_sys::Reflect::set(
        &window,
        &JsValue::from_str("__SERIALIZED_ERRORS"),
        &js_sys::Array::new(),
    )
    .unwrap_throw();
    js_sys::Reflect::set(
        &window,
        &JsValue::from_str("__INCOMPLETE_CHUNKS"),
        &js_sys::Array::new(),
    )
    .unwrap_throw();
    js_sys::Reflect::set(
        &window,
        &JsValue::from_str("__RESOLVED_RESOURCES"),
        &js_sys::Object::new(),
    )
    .unwrap_throw();
}

fn mount_point(ssr_html: &str) -> HtmlElement {
    init_hydration_globals();
    let document = web_sys::window().unwrap_throw().document().unwrap_throw();
    let root = document
        .create_element("div")
        .unwrap_throw()
        .dyn_into::<HtmlElement>()
        .unwrap_throw();
    root.set_inner_html(ssr_html);
    document
        .body()
        .unwrap_throw()
        .append_child(&root)
        .unwrap_throw();
    root
}

fn text_of(root: &HtmlElement, selector: &str) -> Option<String> {
    root.query_selector(selector)
        .unwrap_throw()
        .and_then(|el| el.text_content())
}

async fn settle() {
    for _ in 0..4 {
        leptos::task::tick().await;
    }
}

#[component]
fn BoundaryChild(
    value: ArcRwSignal<Result<&'static str, TestError>>,
    extra: ArcRwSignal<usize>,
    cleanups: Arc<AtomicUsize>,
) -> impl IntoView {
    let local = RwSignal::new(10usize);
    Owner::on_cleanup({
        let cleanups = Arc::clone(&cleanups);
        move || {
            cleanups.fetch_add(1, Ordering::SeqCst);
        }
    });
    let inner_registered = Arc::new(AtomicUsize::new(0));
    view! {
        <p id="ok">
            {move || {
                if inner_registered.swap(1, Ordering::SeqCst) == 0 {
                    // Register a cleanup on the child RenderEffect's parent chain during
                    // initial hydration: if ErrorBoundaryView hydrated children inside its
                    // own RenderEffect, the first error transition would fire this cleanup
                    // while retaining `state.children` for recovery.
                    let cleanups = Arc::clone(&cleanups);
                    if let Some(parent) = Owner::current().and_then(|o| o.parent()) {
                        parent.with(|| {
                            Owner::on_cleanup(move || {
                                cleanups.fetch_add(1, Ordering::SeqCst);
                            });
                        });
                    }
                }
                value
                    .get()
                    .map(|label| format!("{label}:{}", local.get() + extra.get()))
            }}
        </p>
    }
}

fn app_view(
    value: ArcRwSignal<Result<&'static str, TestError>>,
    extra: ArcRwSignal<usize>,
    cleanups: Arc<AtomicUsize>,
) -> impl IntoView {
    view! {
        <ErrorBoundary fallback=move |errors| {
            view! {
                <p id="err">
                    {move || {
                        errors
                            .get()
                            .into_iter()
                            .map(|(_, e)| e.to_string())
                            .collect::<Vec<_>>()
                            .join(",")
                    }}
                </p>
            }
        }>
            <BoundaryChild value=value extra=extra cleanups=cleanups/>
        </ErrorBoundary>
    }
}

#[wasm_bindgen_test(async)]
async fn hydrated_error_boundary_keeps_children_owner_alive_across_errors() {
    let value = ArcRwSignal::new(Ok("init"));
    let extra = ArcRwSignal::new(0usize);
    let ssr_html = "<p id=\"ok\">init:10<!></p>";

    let root = mount_point(&ssr_html);
    let cleanups = Arc::new(AtomicUsize::new(0));
    let mounted = hydrate_from(root.clone(), {
        let value = value.clone();
        let extra = extra.clone();
        let cleanups = Arc::clone(&cleanups);
        move || app_view(value, extra, cleanups)
    });

    settle().await;
    assert_eq!(text_of(&root, "#ok").as_deref(), Some("init:10"));
    assert_eq!(text_of(&root, "#err"), None);

    // Trigger first error: fallback mounts, retained children unmount without disposal.
    value.set(Err(TestError("boom1")));
    settle().await;
    assert_eq!(text_of(&root, "#err").as_deref(), Some("boom1"));
    assert_eq!(text_of(&root, "#ok"), None);
    assert_eq!(cleanups.load(Ordering::SeqCst), 0);

    // Recover from error and update reactive state read inside children.
    extra.set(5);
    value.set(Ok("recovered"));
    settle().await;
    assert_eq!(text_of(&root, "#ok").as_deref(), Some("recovered:15"));
    assert_eq!(text_of(&root, "#err"), None);
    assert_eq!(cleanups.load(Ordering::SeqCst), 0);

    // Trigger a second error after recovery (matches websocket echo_client_error flow).
    value.set(Err(TestError("boom2")));
    settle().await;
    assert_eq!(text_of(&root, "#err").as_deref(), Some("boom2"));
    assert_eq!(text_of(&root, "#ok"), None);

    drop(mounted);
    assert_eq!(cleanups.load(Ordering::SeqCst), 2);
    root.remove();
}

#[wasm_bindgen_test(async)]
async fn async_hydrated_error_boundary_keeps_children_owner_alive_across_errors()
 {
    let value = ArcRwSignal::new(Ok("init"));
    let extra = ArcRwSignal::new(0usize);
    let ssr_html = "<p id=\"ok\">init:10<!></p>";

    let root = mount_point(&ssr_html);
    let cleanups = Arc::new(AtomicUsize::new(0));
    let mounted = hydrate_from_async(root.clone(), {
        let value = value.clone();
        let extra = extra.clone();
        let cleanups = Arc::clone(&cleanups);
        move || app_view(value, extra, cleanups)
    })
    .await;

    settle().await;
    assert_eq!(text_of(&root, "#ok").as_deref(), Some("init:10"));

    value.set(Err(TestError("boom1")));
    settle().await;
    assert_eq!(text_of(&root, "#err").as_deref(), Some("boom1"));
    assert_eq!(cleanups.load(Ordering::SeqCst), 0);

    extra.set(7);
    value.set(Ok("recovered"));
    settle().await;
    assert_eq!(text_of(&root, "#ok").as_deref(), Some("recovered:17"));
    assert_eq!(text_of(&root, "#err"), None);
    assert_eq!(cleanups.load(Ordering::SeqCst), 0);

    value.set(Err(TestError("boom2")));
    settle().await;
    assert_eq!(text_of(&root, "#err").as_deref(), Some("boom2"));

    drop(mounted);
    assert_eq!(cleanups.load(Ordering::SeqCst), 2);
    root.remove();
}
