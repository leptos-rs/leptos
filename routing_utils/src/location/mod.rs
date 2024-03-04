use crate::params::Params;
use alloc::string::String;
use core::fmt::Debug;
use wasm_bindgen::JsValue;

mod browser;
mod server;
pub use browser::*;
pub use server::*;

pub(crate) const BASE: &str = "https://leptos.dev";

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Url {
    origin: String,
    path: String,
    search: String,
    search_params: Params<String>,
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

    pub fn search_params(&self) -> &Params<String> {
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

    fn current(&self) -> Result<Url, Self::Error>;

    /// Sets up any global event listeners or other initialization needed.
    fn init(&self);

    fn set_navigation_hook(&mut self, cb: impl Fn(Url) + 'static);

    /// Navigate to a new location.
    fn navigate(&self, loc: &LocationChange);

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
