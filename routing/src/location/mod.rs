use crate::params::Params;
use std::fmt::Debug;
use wasm_bindgen::JsValue;

mod server;
pub use server::*;

pub(crate) const BASE: &str = "https://leptos.dev";

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Url {
    pub origin: String,
    pub pathname: String,
    pub search: String,
    pub search_params: Params<String>,
    pub hash: String,
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

    /// Sets up any global event listeners or other initialization needed.
    fn init(&self);

    /// Returns the current URL.
    fn try_to_url(&self) -> Result<Url, Self::Error>;

    fn set_navigation_hook(&mut self, cb: impl Fn(Url) + 'static);

    /// Navigate to a new location.
    fn navigate(&self, loc: &LocationChange);
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
