#![allow(missing_docs)]

use any_spawner::Executor;
use core::fmt::Debug;
use dyn_clone::DynClone;
use js_sys::Reflect;
use leptos::{prelude::*, server::ServerActionError};
use reactive_graph::{
    computed::Memo,
    owner::provide_context,
    signal::{ArcRwSignal, ReadSignal},
    traits::With,
};
use send_wrapper::SendWrapper;
use std::{borrow::Cow, future::Future, ops::Deref};
use tachys::dom::window;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Event, HtmlAnchorElement, MouseEvent};

mod hash;
mod history;
mod server;
use crate::{components::RouterContext, params::ParamsMap};
pub use hash::*;
pub use history::*;
pub use server::*;

pub(crate) const BASE: &str = "https://leptos.dev";

// maybe have two types, router url and browser url. Because I think type safety would be worth it here. Currently it is completely unclear where you handle what (as the are identical)

/// The url that is shown in the browser address bar
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserUrl(pub url::Url);

impl Deref for BrowserUrl {
    type Target = url::Url;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl BrowserUrl {
    pub fn parse(url: &str) -> Result<Self, url::ParseError> {
        Ok(Self(url::Url::parse(url)?))
    }
}

/// The url that is used for routing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouterUrl(pub url::Url);

impl Deref for RouterUrl {
    type Target = url::Url;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl RouterUrl {
    pub fn parse(url: &str) -> Result<Self, url::ParseError> {
        Ok(Self(url::Url::parse(url)?))
    }
}

/// A reactive description of the current URL, containing equivalents to the local parts of
/// the browser's [`Location`](https://developer.mozilla.org/en-US/docs/Web/API/Location).
#[derive(Debug, Clone, PartialEq)]
pub struct RouterLocation {
    pub url: Memo<RouterUrl>,
    /// The [`state`](https://developer.mozilla.org/en-US/docs/Web/API/History/state) at the top of the history stack.
    pub state: ReadSignal<State>,
}

impl RouterLocation {
    pub(crate) fn new(
        url: impl Into<ReadSignal<RouterUrl>>,
        state: impl Into<ReadSignal<State>>,
    ) -> Self {
        let url = url.into();
        let state = state.into();
        RouterLocation {
            url: Memo::new(move |_| url.get()),
            state,
        }
    }
}

/// A description of a navigation.
#[derive(Debug, Clone, PartialEq)]
pub struct LocationChange {
    /// The new URL.
    pub value: RouterUrl,
    /// If true, the new location will replace the current one in the history stack, i.e.,
    /// clicking the "back" button will not return to the current location.
    pub replace: bool,
    /// If true, the router will scroll to the top of the page at the end of the navigation.
    pub scroll: bool,
    /// The [`state`](https://developer.mozilla.org/en-US/docs/Web/API/History/state) that will be added during navigation.
    pub state: State,
}

dyn_clone::clone_trait_object!(Routing<Error = JsValue>);

pub trait Routing: DynClone + Send + Sync + 'static {
    type Error: Debug;

    fn as_url(&self) -> &ArcRwSignal<RouterUrl>;

    /// Sets up any global event listeners or other initialization needed.
    fn init(&self, base: Option<Cow<'static, str>>);

    /// Should be called after a navigation when all route components and data have been loaded and
    /// the URL can be updated.
    fn ready_to_complete(&self);

    /// Update the browser's history to reflect a new location.
    fn complete_navigation(&self, loc: &LocationChange);

    /// Whether we are currently in a "back" navigation.
    fn is_back(&self) -> ReadSignal<bool>;

    fn parse(&self, url: &str) -> Result<RouterUrl, Self::Error> {
        self.parse_with_base(url, BASE)
    }

    fn parse_with_base(
        &self,
        url: &str,
        base: &str,
    ) -> Result<RouterUrl, Self::Error>;

    fn redirect(&self, loc: &str);
}

impl Routing for Box<dyn Routing<Error = JsValue> + '_> {
    type Error = JsValue;

    fn as_url(&self) -> &ArcRwSignal<RouterUrl> {
        (**self).as_url()
    }

    fn init(&self, base: Option<Cow<'static, str>>) {
        (**self).init(base)
    }

    fn ready_to_complete(&self) {
        (**self).ready_to_complete();
    }

    fn complete_navigation(&self, loc: &LocationChange) {
        (**self).complete_navigation(loc);
    }

    fn is_back(&self) -> ReadSignal<bool> {
        (**self).is_back()
    }

    fn parse_with_base(
        &self,
        url: &str,
        base: &str,
    ) -> Result<RouterUrl, Self::Error> {
        (**self).parse_with_base(url, base)
    }

    fn redirect(&self, loc: &str) {
        (**self).redirect(loc);
    }
}

pub trait RoutingProvider: Routing + Clone {
    fn new() -> Result<Self, Self::Error>;

    fn current() -> Result<RouterUrl, Self::Error>;
}

#[derive(Debug, Clone, Default)]
pub struct State(Option<SendWrapper<JsValue>>);

impl State {
    pub fn new(state: Option<JsValue>) -> Self {
        Self(state.map(SendWrapper::new))
    }

    pub fn to_js_value(&self) -> JsValue {
        match &self.0 {
            Some(v) => v.clone().take(),
            None => JsValue::UNDEFINED,
        }
    }
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_ref().map(|n| n.as_ref())
            == other.0.as_ref().map(|n| n.as_ref())
    }
}

impl<T> From<T> for State
where
    T: Into<JsValue>,
{
    fn from(value: T) -> Self {
        State::new(Some(value.into()))
    }
}

pub(crate) fn handle_anchor_click<NavFn, NavFut>(
    router_base: Option<Cow<'static, str>>,
    routing: Box<dyn Routing<Error = JsValue>>,
    navigate: NavFn,
) -> Box<dyn Fn(Event) -> Result<(), JsValue>>
where
    NavFn: Fn(RouterUrl, LocationChange) -> NavFut + 'static,
    NavFut: Future<Output = ()> + 'static,
{
    let router_base = router_base.unwrap_or_default();

    Box::new(move |ev: Event| {
        let ev = ev.unchecked_into::<MouseEvent>();
        let browser_url =
            BrowserUrl::parse(&window().location().href()?).unwrap();
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

            // here?
            let url = routing
                .parse_with_base(
                    href.as_str(),
                    &browser_url.origin().unicode_serialization(),
                )
                .unwrap();
            let path_name = url.path();

            // let browser handle this event if it leaves our domain
            // or our base path
            // this probably means we can rely on the assumption that inside our base path we can manipulate urls and outside we can't
            if url.origin() != browser_url.origin()
                || (!router_base.is_empty()
                    && !path_name.is_empty()
                    // NOTE: the two `to_lowercase()` calls here added a total of about 14kb to
                    // release binary size, for limited gain
                    && !path_name.starts_with(&*router_base))
            {
                return Ok(());
            }

            // here we should know whether it is a client side navigation, so copy the part above?

            // we've passed all the checks to navigate on the client side, so we prevent the
            // default behavior of the click
            ev.prevent_default();
            let to = path_name
                + if url.query().is_none() { "" } else { "?" }
                + &url.query().unwrap_or_default()
                + &url.fragment().unwrap_or_default();
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
                scroll: !a.has_attribute("noscroll")
                    && !a.has_attribute("data-noscroll"),
                state: State::new(state),
            };

            Executor::spawn_local(navigate(url, change));
        }

        Ok(())
    })
}
