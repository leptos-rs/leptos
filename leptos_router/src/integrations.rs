use leptos_dom::{document, window, window_event_listener};
use leptos_reactive::{create_signal, ReadSignal, Scope, WriteSignal};

use crate::{LocationChange, State};

pub(crate) fn normalize(cx: Scope) -> (ReadSignal<LocationChange>, WriteSignal<LocationChange>) {
    let (loc, set_loc) = create_signal(cx, location());
    notify(Box::new(move || set_loc.update(|l| *l = location())));
    (loc, set_loc)
}

fn location() -> LocationChange {
    let loc = leptos_dom::location();
    LocationChange {
        value: loc.pathname().unwrap_or_default()
            + &loc.search().unwrap_or_default()
            + &loc.hash().unwrap_or_default(),
        replace: true,
        scroll: true,
        state: State(None), // TODO
    }
}

fn notify(f: impl Fn()) {
    window_event_listener("popstate", |_| f());
}

pub(crate) fn navigate(loc: &LocationChange) {
    let history = window().history().unwrap();
    if loc.replace {
        log::debug!("replacing state");
        history.replace_state_with_url(&loc.state.to_js_value(), "", Some(&loc.value));
    } else {
        log::debug!("pushing state");
        history.push_state_with_url(&loc.state.to_js_value(), "", Some(&loc.value));
    }
    // scroll to el
    if let Ok(hash) = leptos_dom::location().hash() {
        if !hash.is_empty() {
            let hash = &hash[1..];
            let el = document().query_selector(&format!("#{}", hash)).unwrap();
            if let Some(el) = el {
                el.scroll_into_view()
            } else if loc.scroll {
                window().scroll_to_with_x_and_y(0.0, 0.0);
            }
        }
    }
}
