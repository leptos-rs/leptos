use leptos_reactive::{create_signal, ReadSignal, Scope};
use wasm_bindgen::UnwrapThrowExt;

mod location;
mod params;
mod state;
mod url;

pub use self::url::*;
pub use location::*;
pub use params::*;
pub use state::*;

pub trait History {
    fn location(&self, cx: Scope) -> ReadSignal<LocationChange>;

    fn navigate(&self, loc: &LocationChange);
}

pub struct BrowserIntegration {}

impl BrowserIntegration {
    fn current() -> LocationChange {
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
}

impl History for BrowserIntegration {
    fn location(&self, cx: Scope) -> ReadSignal<LocationChange> {
        let (location, set_location) = create_signal(cx, Self::current());

        leptos_dom::window_event_listener("popstate", move |_| {
            set_location(|change| *change = Self::current());
        });

        location
    }

    fn navigate(&self, loc: &LocationChange) {
        let history = leptos_dom::window().history().unwrap();
        if loc.replace {
            log::debug!("replacing state");
            history
                .replace_state_with_url(&loc.state.to_js_value(), "", Some(&loc.value))
                .unwrap_throw();
        } else {
            log::debug!("pushing state");
            history
                .push_state_with_url(&loc.state.to_js_value(), "", Some(&loc.value))
                .unwrap_throw();
        }
        // scroll to el
        if let Ok(hash) = leptos_dom::location().hash() {
            if !hash.is_empty() {
                let hash = &hash[1..];
                let el = leptos_dom::document()
                    .query_selector(&format!("#{}", hash))
                    .unwrap();
                if let Some(el) = el {
                    el.scroll_into_view()
                } else if loc.scroll {
                    leptos_dom::window().scroll_to_with_x_and_y(0.0, 0.0);
                }
            }
        }
    }
}
