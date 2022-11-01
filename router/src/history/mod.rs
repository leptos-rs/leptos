use std::rc::Rc;

use leptos::*;

mod location;
mod params;
mod state;
mod url;

pub use self::url::*;
pub use location::*;
pub use params::*;
pub use state::*;

impl std::fmt::Debug for RouterIntegrationContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RouterIntegrationContext").finish()
    }
}

pub trait History {
    fn location(&self, cx: Scope) -> ReadSignal<LocationChange>;

    fn navigate(&self, loc: &LocationChange);
}

#[cfg(not(feature = "ssr"))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BrowserIntegration {}

#[cfg(not(feature = "ssr"))]
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

#[cfg(not(feature = "ssr"))]
impl History for BrowserIntegration {
    fn location(&self, cx: Scope) -> ReadSignal<LocationChange> {
        use crate::{NavigateOptions, RouterContext};

        let (location, set_location) = create_signal(cx, Self::current());

        leptos_dom::window_event_listener("popstate", move |_| {
            log::debug!(
                "[BrowserIntegration::location] popstate fired {:#?}",
                Self::current()
            );
            let router = use_context::<RouterContext>(cx);
            if let Some(router) = router {
                let change = Self::current();
                if let Err(e) = router.inner.navigate_from_route(
                    &change.value,
                    &NavigateOptions {
                        resolve: false,
                        replace: change.replace,
                        scroll: change.scroll,
                        state: change.state,
                    },
                ) {
                    log::error!("{e:#?}");
                }
                set_location(Self::current());
            } else {
                log::warn!("RouterContext not found");
            }
        });

        location
    }

    fn navigate(&self, loc: &LocationChange) {
        let history = leptos_dom::window().history().unwrap_throw();

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
                if let Some(el) = el {
                    el.scroll_into_view()
                } else if loc.scroll {
                    leptos_dom::window().scroll_to_with_x_and_y(0.0, 0.0);
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct RouterIntegrationContext(pub Rc<dyn History>);

impl RouterIntegrationContext {
    pub fn new(history: impl History + 'static) -> Self {
        Self(Rc::new(history))
    }
}

impl History for RouterIntegrationContext {
    fn location(&self, cx: Scope) -> ReadSignal<LocationChange> {
        self.0.location(cx)
    }

    fn navigate(&self, loc: &LocationChange) {
        self.0.navigate(loc)
    }
}

#[derive(Clone, Debug)]
pub struct ServerIntegration {
    pub path: String,
}

impl History for ServerIntegration {
    fn location(&self, cx: leptos::Scope) -> ReadSignal<LocationChange> {
        create_signal(
            cx,
            LocationChange {
                value: self.path.clone(),
                replace: false,
                scroll: true,
                state: State(None),
            },
        )
        .0
    }

    fn navigate(&self, _loc: &LocationChange) {}
}
