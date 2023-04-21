use super::params::ParamsMap;
use crate::{State, Url};
use leptos::*;

/// Creates a reactive location from the given path and state.
pub fn create_location(
    cx: Scope,
    path: ReadSignal<String>,
    state: ReadSignal<State>,
) -> Location {
    let url = create_memo(cx, move |prev: Option<&Url>| {
        path.with(|path| match Url::try_from(path.as_str()) {
            Ok(url) => url,
            Err(e) => {
                leptos::error!("[Leptos Router] Invalid path {path}\n\n{e:?}");
                prev.cloned().unwrap()
            }
        })
    });

    let pathname =
        create_memo(cx, move |_| url.with(|url| url.pathname.clone()));
    let search = create_memo(cx, move |_| url.with(|url| url.search.clone()));
    let hash = create_memo(cx, move |_| url.with(|url| url.hash.clone()));
    let query =
        create_memo(cx, move |_| url.with(|url| url.search_params.clone()));

    Location {
        pathname,
        search,
        hash,
        query,
        state,
    }
}

/// A reactive description of the current URL, containing equivalents to the local parts of
/// the browser's [`Location`](https://developer.mozilla.org/en-US/docs/Web/API/Location).
#[derive(Debug, Clone, PartialEq)]
pub struct Location {
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
