//! Helpers shared by the router's browser tests.
//!
//! Tests in one test binary share a page, so [`mount`] mounts each app under
//! an owner of its own and [`TestApp`] disposes of it when dropped: that
//! removes the window listeners the router installs, which would otherwise
//! keep handling link clicks for routers of earlier tests. A test that fails
//! aborts without dropping its app, so `mount` also disposes of any app still
//! mounted.

#![allow(dead_code)]

use futures::channel::oneshot;
use leptos::{mount::mount_to, prelude::*};
use leptos_router::{NavigateOptions, hooks::use_navigate};
use std::{any::Any, cell::RefCell, future::Future, mem, time::Duration};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::HtmlElement;

type Navigate = Box<dyn Fn(&str, NavigateOptions)>;

thread_local! {
    static GATES: RefCell<Vec<(&'static str, oneshot::Sender<()>)>> =
        const { RefCell::new(Vec::new()) };
    static NAVIGATE: RefCell<Option<Navigate>> = const { RefCell::new(None) };
    static IS_ROUTING: RefCell<Option<ReadSignal<bool>>> =
        const { RefCell::new(None) };
    static MOUNTED: RefCell<Vec<(Owner, HtmlElement)>> =
        const { RefCell::new(Vec::new()) };
}

fn dispose(owner: &Owner, root: &HtmlElement) {
    owner.cleanup();
    root.remove();
}

/// A future that stays pending until the test releases the gates named
/// `name`.
pub fn gate(name: &'static str) -> impl Future<Output = ()> + Send + 'static {
    let (tx, rx) = oneshot::channel();
    GATES.with(|gates| gates.borrow_mut().push((name, tx)));
    async move {
        _ = rx.await;
    }
}

/// Releases every pending gate named `name`.
pub fn release(name: &str) {
    let released = GATES.with(|gates| {
        let (released, pending) = mem::take(&mut *gates.borrow_mut())
            .into_iter()
            .partition::<Vec<_>, _>(|(gate, _)| *gate == name);
        *gates.borrow_mut() = pending;
        released
    });
    for (_, tx) in released {
        _ = tx.send(());
    }
}

/// Releases only the oldest pending gate named `name`.
pub fn release_first(name: &str) {
    let released = GATES.with(|gates| {
        let mut gates = gates.borrow_mut();
        let index = gates.iter().position(|(gate, _)| *gate == name)?;
        Some(gates.remove(index))
    });
    if let Some((_, tx)) = released {
        _ = tx.send(());
    }
}

/// How many gates named `name` have been created and not released yet.
pub fn pending(name: &str) -> usize {
    GATES.with(|gates| {
        gates
            .borrow()
            .iter()
            .filter(|(gate, _)| *gate == name)
            .count()
    })
}

/// A setter for `<Router set_is_routing>`, whose current value
/// [`is_routing`] returns.
pub fn routing_setter() -> WriteSignal<bool> {
    let (is_routing, set_is_routing) = signal(false);
    IS_ROUTING.with(|signal| *signal.borrow_mut() = Some(is_routing));
    set_is_routing
}

/// The value last set through the app's [`routing_setter`].
pub fn is_routing() -> bool {
    IS_ROUTING.with(|signal| {
        signal
            .borrow()
            .expect("the app does not use routing_setter()")
            .get_untracked()
    })
}

/// Lets [`navigate`] use the router's navigate function: render it inside
/// `<Router>`.
#[component]
pub fn CaptureNavigate() -> impl IntoView {
    let navigate = use_navigate();
    NAVIGATE.with(|nav| *nav.borrow_mut() = Some(Box::new(navigate)));
}

/// Navigates like `use_navigate()` does, which pushes the new URL at once.
pub fn navigate(path: &str) {
    NAVIGATE.with(|nav| {
        let nav = nav.borrow();
        let nav = nav.as_ref().expect("the app has no <CaptureNavigate/>");
        nav(path, NavigateOptions::default())
    });
}

/// Replaces the browser URL without navigating; call it before [`mount`].
pub fn start_at(path: &str) {
    window()
        .history()
        .unwrap()
        .replace_state_with_url(&JsValue::NULL, "", Some(path))
        .unwrap();
}

/// The path in the browser's address bar.
pub fn pathname() -> String {
    window().location().pathname().unwrap()
}

/// Resolves once the microtasks queued so far, and those they queue in turn,
/// have run: the router does all of its work in microtasks.
pub async fn settle() {
    sleep(0).await;
}

/// Resolves after `ms` milliseconds.
pub async fn sleep(ms: u64) {
    let (tx, rx) = oneshot::channel();
    set_timeout(
        move || {
            _ = tx.send(());
        },
        Duration::from_millis(ms),
    )
    .expect("could not set a timeout");
    _ = rx.await;
}

/// An app mounted by [`mount`]; unmounted and disposed of when dropped.
pub struct TestApp {
    root: HtmlElement,
    owner: Owner,
    handle: Option<Box<dyn Any>>,
}

/// Mounts `app` into a new element at the end of the page.
pub fn mount<F, N>(app: F) -> TestApp
where
    F: FnOnce() -> N + 'static,
    N: IntoView + 'static,
{
    for (owner, root) in
        MOUNTED.with(|mounted| mem::take(&mut *mounted.borrow_mut()))
    {
        dispose(&owner, &root);
    }
    GATES.with(|gates| gates.borrow_mut().clear());
    NAVIGATE.with(|nav| *nav.borrow_mut() = None);
    IS_ROUTING.with(|signal| *signal.borrow_mut() = None);

    let root = document()
        .create_element("div")
        .unwrap()
        .unchecked_into::<HtmlElement>();
    document().body().unwrap().append_child(&root).unwrap();
    let owner = Owner::new();
    MOUNTED.with(|mounted| {
        mounted.borrow_mut().push((owner.clone(), root.clone()))
    });
    let handle = owner.with(|| mount_to(root.clone(), app));
    TestApp {
        root,
        owner,
        handle: Some(Box::new(handle)),
    }
}

impl TestApp {
    /// The text of the first element matching `selector`, if any.
    pub fn text(&self, selector: &str) -> Option<String> {
        self.root
            .query_selector(selector)
            .unwrap()
            .map(|el| el.text_content().unwrap_or_default())
    }

    /// Whether an element matches `selector`.
    pub fn has(&self, selector: &str) -> bool {
        self.root.query_selector(selector).unwrap().is_some()
    }

    /// Clicks the first element matching `selector`, like a user would.
    pub fn click(&self, selector: &str) {
        self.root
            .query_selector(selector)
            .unwrap()
            .unwrap_or_else(|| panic!("nothing matches {selector}"))
            .unchecked_into::<HtmlElement>()
            .click();
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        drop(self.handle.take());
        MOUNTED.with(|mounted| {
            mounted
                .borrow_mut()
                .retain(|(owner, _)| *owner != self.owner)
        });
        dispose(&self.owner, &self.root);
    }
}
