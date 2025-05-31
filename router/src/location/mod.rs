#![allow(missing_docs)]

use any_spawner::Executor;
use core::fmt::Debug;
use dyn_clone::DynClone;
use js_sys::Reflect;
use leptos::{prelude::use_context, server::ServerActionError};
use reactive_graph::{
    computed::Memo,
    owner::provide_context,
    signal::{ArcRwSignal, ReadSignal},
    traits::With,
};
use send_wrapper::SendWrapper;
use std::{borrow::Cow, future::Future};
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

pub struct MyBrowserUrl(pub url::Url);

/// The url that is shown in the browser address bar
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BrowserUrl {
    origin: String,
    path: String,
    search: String,
    search_params: ParamsMap,
    hash: String,
}

impl BrowserUrl {
    pub fn origin(&self) -> &str {
        &self.origin
    }

    pub fn origin_mut(&mut self) -> &mut String {
        &mut self.origin
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn path_mut(&mut self) -> &mut str {
        &mut self.path
    }

    pub fn search(&self) -> &str {
        &self.search
    }

    pub fn search_mut(&mut self) -> &mut String {
        &mut self.search
    }

    pub fn search_params(&self) -> &ParamsMap {
        &self.search_params
    }

    pub fn search_params_mut(&mut self) -> &mut ParamsMap {
        &mut self.search_params
    }

    pub fn hash(&self) -> &str {
        &self.hash
    }

    pub fn hash_mut(&mut self) -> &mut String {
        &mut self.hash
    }

    pub fn provide_server_action_error(&self) {
        let search_params = self.search_params();
        if let (Some(err), Some(path)) = (
            search_params.get_str("__err"),
            search_params.get_str("__path"),
        ) {
            provide_context(ServerActionError::new(path, err))
        }
    }

    pub(crate) fn to_full_path(&self) -> String {
        let mut path = self.path.to_string();
        if !self.search.is_empty() {
            path.push('?');
            path.push_str(&self.search);
        }
        if !self.hash.is_empty() {
            if !self.hash.starts_with('#') {
                path.push('#');
            }
            path.push_str(&self.hash);
        }
        path
    }

    pub fn escape(s: &str) -> String {
        #[cfg(not(feature = "ssr"))]
        {
            js_sys::encode_uri_component(s).as_string().unwrap()
        }
        #[cfg(feature = "ssr")]
        {
            percent_encoding::utf8_percent_encode(
                s,
                percent_encoding::NON_ALPHANUMERIC,
            )
            .to_string()
        }
    }

    pub fn unescape(s: &str) -> String {
        #[cfg(feature = "ssr")]
        {
            percent_encoding::percent_decode_str(s)
                .decode_utf8()
                .unwrap()
                .to_string()
        }

        #[cfg(not(feature = "ssr"))]
        {
            match js_sys::decode_uri_component(s) {
                Ok(v) => v.into(),
                Err(_) => s.into(),
            }
        }
    }

    pub fn unescape_minimal(s: &str) -> String {
        #[cfg(not(feature = "ssr"))]
        {
            match js_sys::decode_uri(s) {
                Ok(v) => v.into(),
                Err(_) => s.into(),
            }
        }

        #[cfg(feature = "ssr")]
        {
            Self::unescape(s)
        }
    }
}

/// The url that is used for routing
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RouterUrl {
    origin: String,
    path: String,
    search: String,
    search_params: ParamsMap,
    hash: String,
}

impl RouterUrl {
    pub fn origin(&self) -> &str {
        &self.origin
    }

    pub fn origin_mut(&mut self) -> &mut String {
        &mut self.origin
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn path_mut(&mut self) -> &mut str {
        &mut self.path
    }

    pub fn search(&self) -> &str {
        &self.search
    }

    pub fn search_mut(&mut self) -> &mut String {
        &mut self.search
    }

    pub fn search_params(&self) -> &ParamsMap {
        &self.search_params
    }

    pub fn search_params_mut(&mut self) -> &mut ParamsMap {
        &mut self.search_params
    }

    pub fn hash(&self) -> &str {
        &self.hash
    }

    pub fn hash_mut(&mut self) -> &mut String {
        &mut self.hash
    }

    pub fn provide_server_action_error(&self) {
        let search_params = self.search_params();
        if let (Some(err), Some(path)) = (
            search_params.get_str("__err"),
            search_params.get_str("__path"),
        ) {
            provide_context(ServerActionError::new(path, err))
        }
    }

    pub(crate) fn to_full_path(&self) -> String {
        let mut path = self.path.to_string();
        if !self.search.is_empty() {
            path.push('?');
            path.push_str(&self.search);
        }
        if !self.hash.is_empty() {
            if !self.hash.starts_with('#') {
                path.push('#');
            }
            path.push_str(&self.hash);
        }
        path
    }

    pub fn escape(s: &str) -> String {
        #[cfg(not(feature = "ssr"))]
        {
            js_sys::encode_uri_component(s).as_string().unwrap()
        }
        #[cfg(feature = "ssr")]
        {
            percent_encoding::utf8_percent_encode(
                s,
                percent_encoding::NON_ALPHANUMERIC,
            )
            .to_string()
        }
    }

    pub fn unescape(s: &str) -> String {
        #[cfg(feature = "ssr")]
        {
            percent_encoding::percent_decode_str(s)
                .decode_utf8()
                .unwrap()
                .to_string()
        }

        #[cfg(not(feature = "ssr"))]
        {
            match js_sys::decode_uri_component(s) {
                Ok(v) => v.into(),
                Err(_) => s.into(),
            }
        }
    }

    pub fn unescape_minimal(s: &str) -> String {
        #[cfg(not(feature = "ssr"))]
        {
            match js_sys::decode_uri(s) {
                Ok(v) => v.into(),
                Err(_) => s.into(),
            }
        }

        #[cfg(feature = "ssr")]
        {
            Self::unescape(s)
        }
    }
}

/// A reactive description of the current URL, containing equivalents to the local parts of
/// the browser's [`Location`](https://developer.mozilla.org/en-US/docs/Web/API/Location).
#[derive(Debug, Clone, PartialEq)]
pub struct RouterLocation {
    /// The path of the URL, not containing the query string or hash fragment.
    pub pathname: Memo<String>,
    /// The raw query string.
    pub search: Memo<String>,
    /// The query string parsed into its key-value pairs.
    pub query: Memo<ParamsMap>,
    /// The hash fragment.
    pub hash: Memo<String>,
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
        let pathname = Memo::new(move |_| url.with(|url| url.path.clone()));
        let search = Memo::new(move |_| url.with(|url| url.search.clone()));
        let hash = Memo::new(move |_| url.with(|url| url.hash.clone()));
        let query =
            Memo::new(move |_| url.with(|url| url.search_params.clone()));
        RouterLocation {
            pathname,
            search,
            query,
            hash,
            state,
        }
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

            // here?
            let url = routing.parse_with_base(href.as_str(), &origin).unwrap();
            let path_name = RouterUrl::unescape_minimal(&url.path);

            // let browser handle this event if it leaves our domain
            // or our base path
            // this probably means we can rely on the assumption that inside our base path we can manipulate urls and outside we can't
            if url.origin != origin
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
                + if url.search.is_empty() { "" } else { "?" }
                + &RouterUrl::unescape(&url.search)
                + &RouterUrl::unescape(&url.hash);
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
