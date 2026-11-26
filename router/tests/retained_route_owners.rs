#![cfg(target_arch = "wasm32")]

use leptos::{mount::mount_to, prelude::*};
use leptos_router::{
    NavigateOptions, StaticSegment,
    components::{FlatRoutes, Route, Router, Routes},
    hooks::{use_navigate, use_query_map},
};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::HtmlElement;

wasm_bindgen_test_configure!(run_in_browser);

type Navigate = Box<dyn Fn(&str, NavigateOptions)>;

thread_local! {
    static NAVIGATE: RefCell<Option<Navigate>> = const { RefCell::new(None) };
    static FALLBACK_BUMP: RefCell<Option<Rc<dyn Fn()>>> =
        const { RefCell::new(None) };
}

#[component]
fn CaptureNavigate() -> impl IntoView {
    let navigate = use_navigate();
    NAVIGATE.with(|slot| *slot.borrow_mut() = Some(Box::new(navigate)));
}

#[component]
fn Home() -> impl IntoView {
    view! { <p id="home">"Home"</p> }
}

#[component]
fn PageA() -> impl IntoView {
    let local = RwSignal::new("A");
    view! { <p id="page-a">{move || local.get()}</p> }
}

#[component]
fn PageB() -> impl IntoView {
    let local = RwSignal::new("B");
    view! { <p id="page-b">{move || local.get()}</p> }
}

fn make_fallback(cleanups: Arc<AtomicUsize>) -> impl IntoView {
    let local = RwSignal::new(0usize);
    let query = use_query_map();
    FALLBACK_BUMP.with(|slot| {
        *slot.borrow_mut() = Some(Rc::new(move || {
            local.update(|n| *n += 1);
        }));
    });
    Owner::on_cleanup(move || {
        cleanups.fetch_add(1, Ordering::SeqCst);
    });
    view! {
        <p id="fallback">
            {move || {
                let q = query.read().get("q").unwrap_or_default();
                format!("{q}:{}", local.get())
            }}
        </p>
    }
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

fn text_of(root: &HtmlElement, selector: &str) -> Option<String> {
    root.query_selector(selector)
        .unwrap_throw()
        .and_then(|el| el.text_content())
}

fn navigate(to: &str) {
    NAVIGATE.with(|slot| {
        let slot = slot.borrow();
        let nav = slot.as_ref().expect("navigate hook captured");
        nav(to, Default::default());
    });
}

fn bump_fallback() {
    FALLBACK_BUMP.with(|slot| {
        let slot = slot.borrow();
        let bump = slot.as_ref().expect("fallback bump captured");
        bump();
    });
}

async fn settle() {
    for _ in 0..4 {
        leptos::task::tick().await;
    }
}

fn cleanup(root: &HtmlElement) {
    root.remove();
    replace_url("/");
    NAVIGATE.with(|slot| *slot.borrow_mut() = None);
    FALLBACK_BUMP.with(|slot| *slot.borrow_mut() = None);
}

#[wasm_bindgen_test(async)]
async fn nested_routes_navigates_across_multiple_routes_and_fallback() {
    replace_url("/");
    let root = mount_point();
    let fallback_cleanups = Arc::new(AtomicUsize::new(0));
    let cleanups_for_view = Arc::clone(&fallback_cleanups);

    let mounted = mount_to(root.clone(), move || {
        let cleanups = Arc::clone(&cleanups_for_view);
        view! {
            <Router>
                <CaptureNavigate/>
                <Routes fallback=move || make_fallback(Arc::clone(&cleanups))>
                    <Route path=StaticSegment("") view=Home/>
                    <Route path=StaticSegment("a") view=PageA/>
                    <Route path=StaticSegment("b") view=PageB/>
                </Routes>
            </Router>
        }
    });

    settle().await;
    assert_eq!(text_of(&root, "#home").as_deref(), Some("Home"));

    navigate("/a");
    settle().await;
    assert_eq!(text_of(&root, "#page-a").as_deref(), Some("A"));
    assert_eq!(text_of(&root, "#home"), None);

    navigate("/b");
    settle().await;
    assert_eq!(text_of(&root, "#page-b").as_deref(), Some("B"));
    assert_eq!(text_of(&root, "#page-a"), None);

    navigate("/missing?q=1");
    settle().await;
    assert_eq!(text_of(&root, "#fallback").as_deref(), Some("1:0"));
    assert_eq!(fallback_cleanups.load(Ordering::SeqCst), 0);

    // Same-path query update reruns `<Routes>` outer closure without rebuilding the fallback.
    navigate("/missing?q=2");
    settle().await;
    bump_fallback();
    settle().await;
    assert_eq!(text_of(&root, "#fallback").as_deref(), Some("2:1"));
    assert_eq!(fallback_cleanups.load(Ordering::SeqCst), 0);

    navigate("/a");
    settle().await;
    assert_eq!(text_of(&root, "#page-a").as_deref(), Some("A"));
    assert_eq!(text_of(&root, "#fallback"), None);
    assert_eq!(fallback_cleanups.load(Ordering::SeqCst), 1);

    drop(mounted);
    cleanup(&root);
}

#[wasm_bindgen_test(async)]
async fn flat_routes_initial_fallback_survives_query_update() {
    replace_url("/missing?q=1");
    let root = mount_point();
    let fallback_cleanups = Arc::new(AtomicUsize::new(0));
    let cleanups_for_view = Arc::clone(&fallback_cleanups);

    let mounted = mount_to(root.clone(), move || {
        let cleanups = Arc::clone(&cleanups_for_view);
        view! {
            <Router>
                <CaptureNavigate/>
                <FlatRoutes fallback=move || make_fallback(Arc::clone(&cleanups))>
                    <Route path=StaticSegment("a") view=PageA/>
                </FlatRoutes>
            </Router>
        }
    });

    settle().await;
    assert_eq!(text_of(&root, "#fallback").as_deref(), Some("1:0"));

    navigate("/missing?q=2");
    settle().await;
    bump_fallback();
    settle().await;
    assert_eq!(text_of(&root, "#fallback").as_deref(), Some("2:1"));
    assert_eq!(fallback_cleanups.load(Ordering::SeqCst), 0);

    navigate("/a");
    settle().await;
    assert_eq!(text_of(&root, "#page-a").as_deref(), Some("A"));
    assert_eq!(fallback_cleanups.load(Ordering::SeqCst), 1);

    drop(mounted);
    cleanup(&root);
}
