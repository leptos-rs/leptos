use std::{any::Any, rc::Rc};

use leptos_reactive::{ReadSignal, Scope};

use crate::{Location, Url, State};

pub fn create_location(cx: Scope, path: ReadSignal<String>, state: ReadSignal<State>) -> Location {
    let url = cx.create_memo(move |prev: Option<&Url>| {
        path.with(|path| match Url::try_from(path.as_str()) {
            Ok(url) => url,
            Err(e) => {
                log::error!("[Leptos Router] Invalid path {path}\n\n{e:?}");
                prev.unwrap().clone()
            }
        })
    });

    let path_name = cx.create_memo(move |_| url.with(|url| url.path_name.clone()));
    let search = cx.create_memo(move |_| url.with(|url| url.search.clone()));
    let hash = cx.create_memo(move |_| url.with(|url| url.hash.clone()));
    let query = cx.create_memo(move |_| url.with(|url| url.search_params()));

    Location {
        path_name,
        search,
        hash,
        query,
    }
}
