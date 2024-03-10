use super::{handle_anchor_click, Location, LocationChange, State, Url, BASE};
use crate::params::Params;
use core::fmt;
use js_sys::{try_iter, Array, JsString, Reflect};
use reactive_graph::{signal::ArcRwSignal, traits::Set};
use std::{borrow::Cow, boxed::Box, rc::Rc, string::String};
use tachys::dom::{document, window};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use web_sys::{Event, HtmlAnchorElement, MouseEvent, UrlSearchParams};

#[derive(Clone)]
pub struct BrowserUrl {
    url: ArcRwSignal<Url>,
}

impl fmt::Debug for BrowserUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BrowserUrl").finish_non_exhaustive()
    }
}

impl BrowserUrl {
    pub fn new() -> Result<Self, JsValue> {
        let url = ArcRwSignal::new(Self::current()?);
        Ok(Self { url })
    }

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

impl Location for BrowserUrl {
    type Error = JsValue;

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
            move |new_url, loc| {
                url.set(new_url);
                async move {
                    Self::complete_navigation(&loc);
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
) -> Result<Params, JsValue> {
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
