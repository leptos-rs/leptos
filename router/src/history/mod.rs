use leptos::*;
use std::rc::Rc;
use wasm_bindgen::UnwrapThrowExt;

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

/// The [Router](crate::Router) relies on a [RouterIntegrationContext], which tells the router
/// how to find things like the current URL, and how to navigate to a new page. The [History] trait
/// can be implemented on any type to provide this information.
pub trait History {
    /// A signal that updates whenever the current location changes.
    fn location(&self, cx: Scope) -> ReadSignal<LocationChange>;

    /// Called to navigate to a new location.
    fn navigate(&self, loc: &LocationChange);
}

/// The default integration when you are running in the browser, which uses
/// the [`History API`](https://developer.mozilla.org/en-US/docs/Web/API/History).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BrowserIntegration {}

impl BrowserIntegration {
    fn current(back: bool) -> LocationChange {
        let loc = leptos_dom::helpers::location();
        LocationChange {
            value: loc.pathname().unwrap_or_default()
                + &loc.search().unwrap_or_default()
                + &loc.hash().unwrap_or_default(),
            replace: true,
            scroll: true,
            state: State(None), // TODO
            back,
        }
    }
}

impl History for BrowserIntegration {
    fn location(&self, cx: Scope) -> ReadSignal<LocationChange> {
        use crate::{NavigateOptions, RouterContext};

        let (location, set_location) = create_signal(cx, Self::current(false));

        leptos::window_event_listener("popstate", move |_| {
            let router = use_context::<RouterContext>(cx);
            if let Some(router) = router {
                let is_back = router.inner.is_back;
                let change = Self::current(true);
                is_back.set(true);
                request_animation_frame(move || {
                    is_back.set(false);
                });
                if let Err(e) = router.inner.navigate_from_route(
                    &change.value,
                    &NavigateOptions {
                        resolve: false,
                        replace: change.replace,
                        scroll: change.scroll,
                        state: change.state,
                    },
                    true,
                ) {
                    leptos::error!("{e:#?}");
                }
                set_location.set(Self::current(true));
            } else {
                leptos::warn!("RouterContext not found");
            }
        });

        location
    }

    fn navigate(&self, loc: &LocationChange) {
        let history = leptos_dom::window().history().unwrap_throw();

        if loc.replace {
            history
                .replace_state_with_url(
                    &loc.state.to_js_value(),
                    "",
                    Some(&loc.value),
                )
                .unwrap_throw();
        } else {
            history
                .push_state_with_url(
                    &loc.state.to_js_value(),
                    "",
                    Some(&loc.value),
                )
                .unwrap_throw();
        }
        // scroll to el
        if let Ok(hash) = leptos_dom::helpers::location().hash() {
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

/// The wrapper type that the [Router](crate::Router) uses to interact with a [History].
/// This is automatically provided in the browser. For the server, it should be provided
/// as a context.
///
/// ```
/// # use leptos_router::*;
/// # use leptos::*;
/// # run_scope(create_runtime(), |cx| {
/// let integration = ServerIntegration {
///     path: "insert/current/path/here".to_string(),
/// };
/// provide_context(cx, RouterIntegrationContext::new(integration));
/// # });
/// ```
#[derive(Clone)]
pub struct RouterIntegrationContext(pub Rc<dyn History>);

impl RouterIntegrationContext {
    /// Creates a new router integration.
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

/// A generic router integration for the server side. All its need is the current path.
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
                back: false,
            },
        )
        .0
    }

    fn navigate(&self, _loc: &LocationChange) {}
}
