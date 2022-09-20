use leptos_reactive::{create_signal, use_context, ReadSignal, Scope};
use wasm_bindgen::UnwrapThrowExt;

mod location;
mod params;
mod state;
mod url;

use crate::{NavigateOptions, RouterContext};

pub use self::url::*;
pub use location::*;
pub use params::*;
pub use state::*;

pub trait History {
    fn location(&self, cx: Scope) -> ReadSignal<LocationChange>;

    fn navigate(&self, loc: &LocationChange);
}

#[cfg(any(feature = "csr", feature = "hydrate"))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BrowserIntegration {}

#[cfg(any(feature = "csr", feature = "hydrate"))]
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

#[cfg(any(feature = "csr", feature = "hydrate"))]
impl History for BrowserIntegration {
    fn location(&self, cx: Scope) -> ReadSignal<LocationChange> {
        let (location, set_location) = create_signal(cx, Self::current());

        leptos_dom::window_event_listener("popstate", move |_| {
            log::debug!(
                "[BrowserIntegration::location] popstate fired {:#?}",
                Self::current()
            );
            let router = use_context::<RouterContext>(cx);
            if let Some(router) = router {
                let change = Self::current();
                match router.inner.navigate_from_route(
                    &change.value,
                    &NavigateOptions {
                        resolve: false,
                        replace: change.replace,
                        scroll: change.scroll,
                        state: change.state,
                    },
                ) {
                    Ok(_) => log::debug!("navigated"),
                    Err(e) => log::error!("{e:#?}"),
                };
                set_location(|change| *change = Self::current());
            } else {
                log::debug!("RouterContext not found");
            }

            //Self::navigate(&Self {}, &Self::current());
            //set_location(|change| *change = Self::current());
        });

        location
    }

    fn navigate(&self, loc: &LocationChange) {
        log::debug!("[BrowserIntegration::navigate] {loc:#?}");
        let history = leptos_dom::window().history().unwrap();

        if loc.replace {
            history
                .replace_state_with_url(&loc.state.to_js_value(), "", Some(&loc.value))
                .unwrap_throw();
        } else {
            history
                .push_state_with_url(&loc.state.to_js_value(), "", Some(&loc.value))
                .unwrap_throw();
        }
        // scroll to el
        if let Ok(hash) = leptos_dom::location().hash() {
            if !hash.is_empty() {
                let hash = js_sys::decode_uri(&hash[1..])
                    .ok()
                    .and_then(|decoded| decoded.as_string())
                    .unwrap_or(hash);
                let el = leptos_dom::document().get_element_by_id(&hash);
                log::debug!("el to scroll to = {hash:?} => {el:?}");
                if let Some(el) = el {
                    el.scroll_into_view()
                } else if loc.scroll {
                    leptos_dom::window().scroll_to_with_x_and_y(0.0, 0.0);
                }
            }
        }
        log::debug!("[BrowserIntegration::navigate 5]");
    }
}
