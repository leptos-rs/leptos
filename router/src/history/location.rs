use leptos::*;

use crate::{State, Url};

use super::params::ParamsMap;

pub fn create_location(cx: Scope, path: ReadSignal<String>, state: ReadSignal<State>) -> Location {
    let url = create_memo(cx, move |prev: Option<Url>| {
        path.with(|path| match Url::try_from(path.as_str()) {
            Ok(url) => url,
            Err(e) => {
                log::error!("[Leptos Router] Invalid path {path}\n\n{e:?}");
                prev.clone().unwrap()
            }
        })
    });

    let pathname = create_memo(cx, move |_| url.with(|url| url.pathname.clone()));
    let search = create_memo(cx, move |_| url.with(|url| url.search.clone()));
    let hash = create_memo(cx, move |_| url.with(|url| url.hash.clone()));
    let query = create_memo(cx, move |_| url.with(|url| url.search_params()));

    Location {
        pathname,
        search,
        hash,
        query,
        state,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Location {
    pub query: Memo<ParamsMap>,
    pub pathname: Memo<String>,
    pub search: Memo<String>,
    pub hash: Memo<String>,
    pub state: ReadSignal<State>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LocationChange {
    pub value: String,
    pub replace: bool,
    pub scroll: bool,
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
