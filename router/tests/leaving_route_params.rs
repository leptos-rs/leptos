#![cfg(target_arch = "wasm32")]

use futures::channel::oneshot;
use leptos::{mount::mount_to, prelude::*};
use leptos_router::{
    Lazy, LazyRoute, NavigateOptions, ParamSegment, StaticSegment,
    components::{Outlet, ParentRoute, Route, Router, Routes},
    hooks::{use_navigate, use_params_map},
    params::ParamsMap,
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
    static NEW_VIEW_PARAMS: RefCell<Option<Memo<ParamsMap>>> =
        const { RefCell::new(None) };
}

struct SlowItem;

/// Stands in for the wasm chunk of a `#[lazy]` route: `<Routes>` keeps the
/// previous route mounted until `preload` resolves, which the test controls.
async fn await_chunk() {
    let (sender, receiver) = oneshot::channel();
    CHUNK_WAITERS.with(|waiters| waiters.borrow_mut().push(sender));
    _ = receiver.await;
}

impl LazyRoute for SlowItem {
    fn data() -> Self {
        Self
    }

    async fn view(_this: Self) -> AnyView {
        view! { <p id="view-item">"View item"</p> }.into_any()
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
fn ViewNew() -> impl IntoView {
    let params = use_params_map();
    NEW_VIEW_PARAMS.with(|slot| *slot.borrow_mut() = Some(params));
    view! { <p id="view-new">"View new"</p> }
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

fn navigate_to(path: &str) -> Result<(), &'static str> {
    NAVIGATE.with(|slot| {
        let slot = slot.borrow();
        let navigate = slot.as_ref().ok_or("navigate hook was not captured")?;
        navigate(path, Default::default());
        Ok(())
    })
}

fn new_view_id_param() -> Result<Option<String>, &'static str> {
    NEW_VIEW_PARAMS.with(|slot| {
        let params =
            slot.borrow().ok_or("new view params were not captured")?;
        Ok(params.with_untracked(|params| params.get("id")))
    })
}

fn resolve_slow_item() -> Result<(), &'static str> {
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
    NEW_VIEW_PARAMS.with(|slot| *slot.borrow_mut() = None);
    CHUNK_WAITERS.with(|waiters| waiters.borrow_mut().clear());
}

#[wasm_bindgen_test(async)]
async fn leaving_route_without_param_does_not_see_destination_param() {
    replace_url("/p/new");
    let root = mount_point();
    let mounted = mount_to(root.clone(), || {
        view! {
            <Router>
                <CaptureNavigate/>
                <Routes fallback=|| "not found">
                    <ParentRoute path=StaticSegment("p") view=Outlet>
                        <Route path=StaticSegment("new") view=ViewNew/>
                        <Route
                            path=(StaticSegment("item"), ParamSegment("id"))
                            view={Lazy::<SlowItem>::new()}
                        />
                    </ParentRoute>
                </Routes>
            </Router>
        }
    });

    let result = async {
        settle().await;
        check(has(&root, "#view-new"), "the new view did not appear")?;
        check(
            new_view_id_param()?.is_none(),
            "the new view had an id before navigation",
        )?;
        navigate_to("/p/item/2")?;
        settle().await;
        check(
            has(&root, "#view-new"),
            "the new view was unmounted before the item route loaded",
        )?;
        check(
            new_view_id_param()?.is_none(),
            "the leaving new view read the destination route's id",
        )?;
        resolve_slow_item()?;
        settle().await;
        check(has(&root, "#view-item"), "the item view did not appear")?;
        check(!has(&root, "#view-new"), "the new view stayed mounted")
    }
    .await;

    drop(mounted);
    cleanup(&root);
    assert!(result.is_ok(), "{}", result.unwrap_err());
}
