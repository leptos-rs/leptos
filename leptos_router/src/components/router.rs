use std::{any::Any, cell::RefCell, future::Future, rc::Rc};

use leptos_core as leptos;
use leptos_dom::{location, window_event_listener, IntoChild};
use leptos_macro::{view, Props};
use leptos_reactive::{
    create_effect, create_render_effect, create_signal, provide_context, use_transition,
    ReadSignal, Scope, WriteSignal,
};
use thiserror::Error;

use crate::{
    create_location, integrations, resolve_path, unescape, DataFunction, Location, LocationChange,
    ParamsMap, RouteContext, State, Url,
};

#[derive(Props)]
pub struct RouterProps<C, D, Fu, T>
where
    C: IntoChild,
    D: Fn(ParamsMap, Location) -> Fu + Clone + 'static,
    Fu: Future<Output = T>,
    T: Any + 'static,
{
    base: Option<String>,
    data: Option<D>,
    children: C,
}

#[allow(non_snake_case)]
pub fn Router<C, D, Fu, T>(cx: Scope, props: RouterProps<C, D, Fu, T>) -> impl IntoChild
where
    C: IntoChild,
    D: Fn(ParamsMap, Location) -> Fu + Clone + 'static,
    Fu: Future<Output = T>,
    T: Any + 'static,
{
    provide_context(
        cx,
        RouterContext::new(
            cx,
            props.base,
            props.data.map(|data| DataFunction::from(data)),
        ),
    );

    log::debug!("provided RouterContext");

    props.children
}

#[derive(Debug, Clone)]
pub struct RouterContext {
    pub(crate) inner: Rc<RouterContextInner>,
}
#[derive(Debug)]
pub(crate) struct RouterContextInner {
    pub location: Location,
    pub base: RouteContext,
    cx: Scope,
    reference: ReadSignal<String>,
    set_reference: WriteSignal<String>,
    referrers: Rc<RefCell<Vec<LocationChange>>>,
    source: ReadSignal<LocationChange>,
    set_source: WriteSignal<LocationChange>,
    state: ReadSignal<State>,
    set_state: WriteSignal<State>,
}

impl RouterContext {
    pub fn new(cx: Scope, base: Option<String>, data: Option<DataFunction>) -> Self {
        let (source, set_source) = integrations::normalize(cx);
        let base = base.unwrap_or_default();
        let base_path = resolve_path("", &base, None);
        if let Some(base_path) = &base_path && source.with(|s| s.value.is_empty()) {
			set_source(|source| *source = LocationChange {
				value: base_path.to_string(),
				replace: true,
				scroll: false,
				state: State(None)
			});
		}
        let (reference, set_reference) = create_signal(cx, source.with(|s| s.value.clone()));
        let (state, set_state) = create_signal(cx, source.with(|s| s.state.clone()));
        let transition = use_transition(cx);
        let location = create_location(cx, reference, state);
        let referrers: Rc<RefCell<Vec<LocationChange>>> = Rc::new(RefCell::new(Vec::new()));

        let base_path = base_path.map(|s| s.to_owned()).unwrap_or_default();
        let base = RouteContext::base(&base_path);

        if let Some(data) = data {
            log::debug!("skipping data fn");
            // TODO
        }

        create_effect(cx, move |_| {
            log::debug!("location.path_name = {:?}", location.path_name.get())
        });

        create_render_effect(cx, move |_| {
            let LocationChange { value, state, .. } = source();
            cx.untrack(move || {
                if value != reference() {
                    transition.start(move || {
                        set_reference(|r| *r = value.clone());
                        set_state(|s| *s = state.clone());
                    });
                }
            });
        });

        let inner = Rc::new(RouterContextInner {
            location,
            base,
            cx,
            reference,
            set_reference,
            referrers,
            source,
            set_source,
            state,
            set_state,
        });

        if cfg!(feature = "browser") {
            window_event_listener("click", {
                let inner = Rc::clone(&inner);
                move |ev| inner.clone().handle_anchor_click(ev)
            });
            // TODO on_cleanup remove event listener
        }

        Self { inner }
    }
}

impl RouterContextInner {
    pub(crate) fn navigate_from_route(
        self: Rc<Self>,
        to: &str,
        options: &NavigateOptions,
    ) -> Result<(), NavigationError> {
        let cx = self.cx;
        let this = Rc::clone(&self);

        // TODO untrack causes an error here
        cx.untrack(move || {
            let resolved_to = if options.resolve {
                this.base.resolve_path(to)
            } else {
                resolve_path("", to, None)
            };

            match resolved_to {
                None => Err(NavigationError::NotRoutable(to.to_string())),
                Some(resolved_to) => {
                    if self.referrers.borrow().len() > 32 {
                        return Err(NavigationError::MaxRedirects);
                    }

                    if resolved_to != (this.reference)() || options.state != (this.state).get() {
                        if cfg!(feature = "server") {
                            // TODO server out
                            self.set_source.update(|source| {
                                *source = LocationChange {
                                    value: resolved_to.to_string(),
                                    replace: options.replace,
                                    scroll: options.scroll,
                                    state: options.state.clone(),
                                }
                            });
                        } else {
                            {
                                self.referrers.borrow_mut().push(LocationChange {
                                    value: resolved_to.to_string(),
                                    replace: options.replace,
                                    scroll: options.scroll,
                                    state: State(None), // TODO state state.get(),
                                });
                            }
                            let len = self.referrers.borrow().len();

                            let transition = use_transition(self.cx);
                            transition.start({
                                let set_reference = self.set_reference;
                                let set_state = self.set_state;
                                let referrers = self.referrers.clone();
                                let this = Rc::clone(&self);
                                move || {
                                    set_reference.update({
                                        let resolved = resolved_to.to_string();
                                        move |r| *r = resolved
                                    });
                                    set_state.update({
                                        let next_state = options.state.clone();
                                        move |state| *state = next_state
                                    });
                                    if referrers.borrow().len() == len {
                                        this.navigate_end(LocationChange {
                                            value: resolved_to.to_string(),
                                            replace: false,
                                            scroll: true,
                                            state: options.state.clone(),
                                        })
                                    }
                                }
                            });
                        }
                    }

                    Ok(())
                }
            }
        })
    }

    pub(crate) fn navigate_end(self: Rc<Self>, next: LocationChange) {
        let first = self.referrers.borrow().get(0).cloned();
        if let Some(first) = first {
            if next.value != first.value || next.state != first.state {
                let next = next.clone();
                self.set_source.update(move |source| {
                    *source = next;
                    source.replace = first.replace;
                    source.scroll = first.scroll;
                })
            }
            self.referrers.borrow_mut().clear();
        }
        integrations::navigate(&next);
    }

    pub(crate) fn handle_anchor_click(self: Rc<Self>, ev: web_sys::Event) {
        use leptos_dom::wasm_bindgen::JsCast;
        let ev = ev.unchecked_into::<web_sys::MouseEvent>();
        /* if ev.default_prevented()
            || ev.button() != 0
            || ev.meta_key()
            || ev.alt_key()
            || ev.ctrl_key()
            || ev.shift_key()
        {
            log::debug!("branch A prevent");
            return;
        } */

        let composed_path = ev.composed_path();
        let mut a: Option<web_sys::HtmlAnchorElement> = None;
        for i in 0..composed_path.length() {
            if let Ok(el) = composed_path
                .get(i)
                .dyn_into::<web_sys::HtmlAnchorElement>()
            {
                a = Some(el);
            }
        }
        if let Some(a) = a {
            let href = a.href();
            let target = a.target();

            // let browser handle this event if link has target,
            // or if it doesn't have href or state
            /* if !target.is_empty() || (href.is_empty() && !a.has_attribute("state")) {
                log::debug!("target or href empty");
                ev.prevent_default();
                return;
            } */

            let rel = a.get_attribute("rel").unwrap_or_default();
            let mut rel = rel.split([' ', '\t']);

            // let browser handle event if it has rel=external or download
            if a.has_attribute("download") || rel.any(|p| p == "external") {
                return;
            }

            let url = Url::try_from(href.as_str()).unwrap();
            let path_name = unescape(&url.path_name);

            // let browser handle this event if it leaves our domain
            // or our base path
            /* if url.origin != leptos_dom::location().origin().unwrap_or_default()
                || (!base_path.is_empty()
                    && !path_name.is_empty()
                    && !path_name
                        .to_lowercase()
                        .starts_with(&base_path.to_lowercase()))
            {
                return;
            } */

            let to = path_name + &unescape(&url.search) + &unescape(&url.hash);
            let state = a.get_attribute("state"); // TODO state

            ev.prevent_default();
            log::debug!("navigate to {to}");

            match self.navigate_from_route(
                &to,
                &NavigateOptions {
                    resolve: false,
                    replace: a.has_attribute("replace"),
                    scroll: !a.has_attribute("noscroll"),
                    state: State(None), // TODO state
                },
            ) {
                Ok(_) => log::debug!("navigated"),
                Err(e) => log::error!("{e:#?}"),
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum NavigationError {
    #[error("Path {0:?} is not routable")]
    NotRoutable(String),
    #[error("Too many redirects")]
    MaxRedirects,
}

pub struct NavigateOptions {
    pub resolve: bool,
    pub replace: bool,
    pub scroll: bool,
    pub state: State,
}

impl Default for NavigateOptions {
    fn default() -> Self {
        Self {
            resolve: true,
            replace: false,
            scroll: true,
            state: State(None),
        }
    }
}
