use any_spawner::Executor;
use core::fmt::Debug;
use js_sys::Reflect;
use reactive_graph::signal::ArcRwSignal;
use std::{borrow::Cow, future::Future};
use tachys::dom::window;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Event, HtmlAnchorElement, MouseEvent};

mod history;
mod server;
use crate::Params;
pub use history::*;
pub use server::*;

pub(crate) const BASE: &str = "https://leptos.dev";

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Url {
    origin: String,
    path: String,
    search: String,
    search_params: Params,
    hash: String,
}

impl Url {
    pub fn origin(&self) -> &str {
        &self.origin
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn search(&self) -> &str {
        &self.search
    }

    pub fn search_params(&self) -> &Params {
        &self.search_params
    }

    pub fn hash(&self) -> &str {
        &self.hash
    }
}

/// A description of a navigation.
#[derive(Debug, Clone, PartialEq)]
pub struct LocationChange {
    /// The new URL.
    pub value: String,
    /// If true, the new location will replace the current one in the history stack, i.e.,
    /// clicking the "back" button will not return to the current location.
    pub replace: bool,
    /// If true, the router will scroll to the top of the page at the end of the navigation.
    pub scroll: bool,
    /// The [`state`](https://developer.mozilla.org/en-US/docs/Web/API/History/state) that will be added during navigation.
    pub state: State,
}

impl Default for LocationChange {
    fn default() -> Self {
        Self {
            value: Default::default(),
            replace: true,
            scroll: true,
            state: Default::default(),
        }
    }
}

pub trait Location {
    type Error: Debug;

    fn as_url(&self) -> &ArcRwSignal<Url>;

    fn current() -> Result<Url, Self::Error>;

    /// Sets up any global event listeners or other initialization needed.
    fn init(&self, base: Option<Cow<'static, str>>);

    /// Update the browser's history to reflect a new location.
    fn complete_navigation(loc: &LocationChange);

    fn parse(url: &str) -> Result<Url, Self::Error> {
        Self::parse_with_base(url, BASE)
    }

    fn parse_with_base(url: &str, base: &str) -> Result<Url, Self::Error>;
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct State(pub Option<JsValue>);

impl State {
    pub fn to_js_value(&self) -> JsValue {
        match &self.0 {
            Some(v) => v.clone(),
            None => JsValue::UNDEFINED,
        }
    }
}

impl<T> From<T> for State
where
    T: Into<JsValue>,
{
    fn from(value: T) -> Self {
        State(Some(value.into()))
    }
}

pub(crate) fn unescape(s: &str) -> String {
    js_sys::decode_uri(s).unwrap().into()
}

pub(crate) fn handle_anchor_click<NavFn, NavFut>(
    router_base: Option<Cow<'static, str>>,
    parse_with_base: fn(&str, &str) -> Result<Url, JsValue>,
    navigate: NavFn,
) -> Box<dyn Fn(Event) -> Result<(), JsValue>>
where
    NavFn: Fn(Url, LocationChange) -> NavFut + 'static,
    NavFut: Future<Output = ()> + 'static,
{
    let router_base = router_base.unwrap_or_default();

    Box::new(move |ev: Event| {
        let ev = ev.unchecked_into::<MouseEvent>();
        let origin = window().location().origin()?;
        if ev.default_prevented()
            || ev.button() != 0
            || ev.meta_key()
            || ev.alt_key()
            || ev.ctrl_key()
            || ev.shift_key()
        {
            return Ok(());
        }

        let composed_path = ev.composed_path();
        let mut a: Option<HtmlAnchorElement> = None;
        for i in 0..composed_path.length() {
            if let Ok(el) = composed_path.get(i).dyn_into::<HtmlAnchorElement>()
            {
                a = Some(el);
            }
        }
        if let Some(a) = a {
            let href = a.href();
            let target = a.target();

            // let browser handle this event if link has target,
            // or if it doesn't have href or state
            // TODO "state" is set as a prop, not an attribute
            if !target.is_empty()
                || (href.is_empty() && !a.has_attribute("state"))
            {
                return Ok(());
            }

            let rel = a.get_attribute("rel").unwrap_or_default();
            let mut rel = rel.split([' ', '\t']);

            // let browser handle event if it has rel=external or download
            if a.has_attribute("download") || rel.any(|p| p == "external") {
                return Ok(());
            }

            let url = parse_with_base(href.as_str(), &origin).unwrap();
            let path_name = unescape(&url.path);
            ev.prevent_default();

            // let browser handle this event if it leaves our domain
            // or our base path
            if url.origin != origin
                || (!router_base.is_empty()
                    && !path_name.is_empty()
                    // NOTE: the two `to_lowercase()` calls here added a total of about 14kb to
                    // release binary size, for limited gain
                    && !path_name.starts_with(&*router_base))
            {
                return Ok(());
            }

            let to = path_name
                + if url.search.is_empty() { "" } else { "?" }
                + &unescape(&url.search)
                + &unescape(&url.hash);
            let state = Reflect::get(&a, &JsValue::from_str("state"))
                .ok()
                .and_then(|value| {
                    if value == JsValue::UNDEFINED {
                        None
                    } else {
                        Some(value)
                    }
                });

            let replace = Reflect::get(&a, &JsValue::from_str("replace"))
                .ok()
                .and_then(|value| value.as_bool())
                .unwrap_or(false);

            let change = LocationChange {
                value: to,
                replace,
                scroll: true,
                state: State(state),
            };

            Executor::spawn_local(navigate(url, change));
        }

        Ok(())
    })
}
