use super::{
    handle_anchor_click, LocationChange, LocationProvider, State, Url, BASE,
};
use crate::{navigate::UseNavigate, params::ParamsMap};
use core::fmt;
use futures::channel::oneshot;
use js_sys::{try_iter, Array, JsString, Reflect};
use or_poisoned::OrPoisoned;
use reactive_graph::{
    signal::ArcRwSignal,
    traits::{ReadUntracked, Set},
};
use std::{
    borrow::Cow,
    boxed::Box,
    cell::RefCell,
    rc::Rc,
    string::String,
    sync::{Arc, Mutex},
};
use tachys::dom::{document, window};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use web_sys::{Event, HtmlAnchorElement, MouseEvent, UrlSearchParams};

#[derive(Clone)]
pub struct BrowserUrl {
    url: ArcRwSignal<Url>,
    pending_navigation: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

impl fmt::Debug for BrowserUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BrowserUrl").finish_non_exhaustive()
    }
}

impl BrowserUrl {
    fn scroll_to_el(loc_scroll: bool) {
        if let Ok(hash) = window().location().hash() {
            if !hash.is_empty() {
                let hash = js_sys::decode_uri(&hash[1..])
                    .ok()
                    .and_then(|decoded| decoded.as_string())
                    .unwrap_or(hash);
                let el = document().get_element_by_id(&hash);
                if let Some(el) = el {
                    el.scroll_into_view();
                    return;
                }
            }
        }

        // scroll to top
        if loc_scroll {
            window().scroll_to_with_x_and_y(0.0, 0.0);
        }
    }
}

impl LocationProvider for BrowserUrl {
    type Error = JsValue;

    fn new() -> Result<Self, JsValue> {
        let url = ArcRwSignal::new(Self::current()?);
        let pending_navigation = Default::default();
        Ok(Self {
            url,
            pending_navigation,
        })
    }

    fn as_url(&self) -> &ArcRwSignal<Url> {
        &self.url
    }

    fn current() -> Result<Url, Self::Error> {
        let location = window().location();
        Ok(Url {
            origin: location.origin()?,
            path: location.pathname()?,
            search: location
                .search()?
                .strip_prefix('?')
                .map(String::from)
                .unwrap_or_default(),
            search_params: search_params_from_web_url(
                &UrlSearchParams::new_with_str(&location.search()?)?,
            )?,
            hash: location.hash()?,
        })
    }

    fn parse_with_base(url: &str, base: &str) -> Result<Url, Self::Error> {
        let location = web_sys::Url::new_with_base(url, base)?;
        Ok(Url {
            origin: location.origin(),
            path: location.pathname(),
            search: location
                .search()
                .strip_prefix('?')
                .map(String::from)
                .unwrap_or_default(),
            search_params: search_params_from_web_url(
                &location.search_params(),
            )?,
            hash: location.hash(),
        })
    }

    fn init(&self, base: Option<Cow<'static, str>>) {
        let window = window();
        let navigate = {
            let url = self.url.clone();
            let pending = Arc::clone(&self.pending_navigation);
            move |new_url: Url, loc| {
                let same_path = {
                    let curr = url.read_untracked();
                    curr.origin() == new_url.origin()
                        && curr.path() == new_url.path()
                };
                url.set(new_url);
                if same_path {
                    Self::complete_navigation(&loc);
                }
                let pending = Arc::clone(&pending);
                async move {
                    if !same_path {
                        let (tx, rx) = oneshot::channel::<()>();
                        *pending.lock().or_poisoned() = Some(tx);
                        // if it has been canceled, ignore
                        // otherwise, complete navigation -- i.e., set URL in address bar
                        if rx.await.is_ok() {
                            Self::complete_navigation(&loc);
                        }
                    }
                }
            }
        };

        let handle_anchor_click =
            handle_anchor_click(base, Self::parse_with_base, navigate);
        let closure = Closure::wrap(Box::new(move |ev: Event| {
            if let Err(e) = handle_anchor_click(ev) {
                #[cfg(feature = "tracing")]
                tracing::error!("{e:?}");
                #[cfg(not(feature = "tracing"))]
                web_sys::console::error_1(&e);
            }
        }) as Box<dyn FnMut(Event)>)
        .into_js_value();
        window
            .add_event_listener_with_callback(
                "click",
                closure.as_ref().unchecked_ref(),
            )
            .expect(
                "couldn't add `click` listener to `window` to handle `<a>` \
                 clicks",
            );

        // handle popstate event (forward/back navigation)
        let cb = {
            let url = self.url.clone();
            move || match Self::current() {
                Ok(new_url) => url.set(new_url),
                Err(e) => {
                    #[cfg(feature = "tracing")]
                    tracing::error!("{e:?}");
                    #[cfg(not(feature = "tracing"))]
                    web_sys::console::error_1(&e);
                }
            }
        };
        let closure =
            Closure::wrap(Box::new(cb) as Box<dyn Fn()>).into_js_value();
        window
            .add_event_listener_with_callback(
                "popstate",
                closure.as_ref().unchecked_ref(),
            )
            .expect("couldn't add `popstate` listener to `window`");
    }

    fn ready_to_complete(&self) {
        if let Some(tx) = self.pending_navigation.lock().or_poisoned().take() {
            tx.send(());
        }
    }

    fn complete_navigation(loc: &LocationChange) {
        let history = window().history().unwrap();

        if loc.replace {
            history
                .replace_state_with_url(
                    &loc.state.to_js_value(),
                    "",
                    Some(&loc.value),
                )
                .unwrap();
        } else {
            // push the "forward direction" marker
            let state = &loc.state.to_js_value();
            history
                .push_state_with_url(state, "", Some(&loc.value))
                .unwrap();
        }
        // scroll to el
        Self::scroll_to_el(loc.scroll);
    }
}

fn search_params_from_web_url(
    params: &web_sys::UrlSearchParams,
) -> Result<ParamsMap, JsValue> {
    try_iter(params)?
        .into_iter()
        .flatten()
        .map(|pair| {
            pair.and_then(|pair| {
                let row = pair.dyn_into::<Array>()?;
                Ok((
                    String::from(row.get(0).dyn_into::<JsString>()?),
                    String::from(row.get(1).dyn_into::<JsString>()?),
                ))
            })
        })
        .collect()
}
