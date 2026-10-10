#![cfg(target_arch = "wasm32")]

use futures::channel::oneshot;
use leptos::{mount::mount_to, prelude::*};
use leptos_router::{
    Lazy, LazyRoute, NavigateOptions, StaticSegment,
    components::{FlatRoutes, Route, Router, Routes},
    hooks::use_navigate,
};
use std::cell::RefCell;
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::HtmlElement;

wasm_bindgen_test_configure!(run_in_browser);

type Navigate = Box<dyn Fn(&str, NavigateOptions)>;

thread_local! {
    static CHUNK_WAITERS: RefCell<Vec<oneshot::Sender<()>>> =
        const { RefCell::new(Vec::new()) };
    static NAVIGATE: RefCell<Option<Navigate>> = const { RefCell::new(None) };
}

struct SlowC;

/// Stands in for the wasm chunk of a `#[lazy]` route: both `preload` (used
/// by `<Routes>`) and `view` (used by `<FlatRoutes>`) wait until the test
/// releases it.
async fn await_chunk() {
    let (sender, receiver) = oneshot::channel();
    CHUNK_WAITERS.with(|waiters| waiters.borrow_mut().push(sender));
    _ = receiver.await;
}

impl LazyRoute for SlowC {
    fn data() -> Self {
        Self
    }

    async fn view(_this: Self) -> AnyView {
        await_chunk().await;
        view! { <p id="view-c">"View C"</p> }.into_any()
    }

    async fn preload() {
        await_chunk().await;
    }
}

#[component]
fn CaptureNavigate() -> impl IntoView {
    let navigate = use_navigate();
    NAVIGATE.with(|slot| *slot.borrow_mut() = Some(Box::new(navigate)));
}

#[component]
fn ViewB() -> impl IntoView {
    view! { <p id="view-b">"View B"</p> }
}

fn replace_url(path: &str) {
    web_sys::window()
        .unwrap_throw()
        .history()
        .unwrap_throw()
        .replace_state_with_url(&JsValue::NULL, "", Some(path))
        .unwrap_throw();
}

fn mount_point() -> HtmlElement {
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
    root
}

fn has(root: &HtmlElement, selector: &str) -> bool {
    root.query_selector(selector).unwrap_throw().is_some()
}

fn navigate_to_b() -> Result<(), &'static str> {
    NAVIGATE.with(|slot| {
        let slot = slot.borrow();
        let navigate = slot.as_ref().ok_or("navigate hook was not captured")?;
        navigate("/b", Default::default());
        Ok(())
    })
}

fn resolve_slow_c() -> Result<(), &'static str> {
    CHUNK_WAITERS.with(|waiters| {
        let waiters = std::mem::take(&mut *waiters.borrow_mut());
        if waiters.is_empty() {
            return Err("slow route was not started");
        }
        for sender in waiters {
            _ = sender.send(());
        }
        Ok(())
    })
}

async fn settle() {
    for _ in 0..4 {
        leptos::task::tick().await;
    }
}

fn check(condition: bool, message: &'static str) -> Result<(), &'static str> {
    condition.then_some(()).ok_or(message)
}

fn cleanup(root: &HtmlElement) {
    root.remove();
    replace_url("/");
    NAVIGATE.with(|slot| *slot.borrow_mut() = None);
    CHUNK_WAITERS.with(|waiters| waiters.borrow_mut().clear());
}

#[wasm_bindgen_test(async)]
async fn flat_routes_initial_view_does_not_replace_newer_navigation() {
    replace_url("/c");
    let root = mount_point();
    let mounted = mount_to(root.clone(), || {
        view! {
            <Router>
                <CaptureNavigate/>
                <FlatRoutes fallback=|| "not found">
                    <Route path=StaticSegment("c") view={Lazy::<SlowC>::new()}/>
                    <Route path=StaticSegment("b") view=ViewB/>
                </FlatRoutes>
            </Router>
        }
    });

    let result = async {
        settle().await;
        check(!has(&root, "#view-b"), "view B appeared before navigation")?;
        check(!has(&root, "#view-c"), "pending view C appeared")?;
        navigate_to_b()?;
        settle().await;
        check(
            has(&root, "#view-b"),
            "view B did not appear after navigation",
        )?;
        resolve_slow_c()?;
        settle().await;
        check(has(&root, "#view-b"), "view B was replaced by stale view C")?;
        check(!has(&root, "#view-c"), "stale view C appeared")
    }
    .await;

    drop(mounted);
    cleanup(&root);
    assert!(result.is_ok(), "{}", result.unwrap_err());
}

#[wasm_bindgen_test(async)]
async fn nested_routes_initial_view_does_not_replace_newer_navigation() {
    replace_url("/c");
    let root = mount_point();
    let mounted = mount_to(root.clone(), || {
        view! {
            <Router>
                <CaptureNavigate/>
                <Routes fallback=|| "not found">
                    <Route path=StaticSegment("c") view={Lazy::<SlowC>::new()}/>
                    <Route path=StaticSegment("b") view=ViewB/>
                </Routes>
            </Router>
        }
    });

    let result = async {
        settle().await;
        check(!has(&root, "#view-b"), "view B appeared before navigation")?;
        check(!has(&root, "#view-c"), "pending view C appeared")?;
        navigate_to_b()?;
        settle().await;
        check(
            has(&root, "#view-b"),
            "view B did not appear after navigation",
        )?;
        resolve_slow_c()?;
        settle().await;
        check(has(&root, "#view-b"), "view B was replaced by stale view C")?;
        check(!has(&root, "#view-c"), "stale view C appeared")
    }
    .await;

    drop(mounted);
    cleanup(&root);
    assert!(result.is_ok(), "{}", result.unwrap_err());
}
