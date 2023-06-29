use crate::{
    create_location, matching::resolve_path, Branch, History, Location,
    LocationChange, RouteContext, RouterIntegrationContext, State,
};
#[cfg(not(feature = "ssr"))]
use crate::{unescape, Url};
use cfg_if::cfg_if;
use leptos::*;
#[cfg(feature = "transition")]
use leptos_reactive::use_transition;
use std::{cell::RefCell, rc::Rc};
use thiserror::Error;
#[cfg(not(feature = "ssr"))]
use wasm_bindgen::JsCast;

/// Provides for client-side and server-side routing. This should usually be somewhere near
/// the root of the application.
#[component]
pub fn Router(
    cx: Scope,
    /// The base URL for the router. Defaults to "".
    #[prop(optional)]
    base: Option<&'static str>,
    /// A fallback that should be shown if no route is matched.
    #[prop(optional)]
    fallback: Option<fn(Scope) -> View>,
    /// A signal that will be set while the navigation process is underway.
    #[prop(optional, into)]
    set_is_routing: Option<SignalSetter<bool>>,
    /// The `<Router/>` should usually wrap your whole page. It can contain
    /// any elements, and should include a [Routes](crate::Routes) component somewhere
    /// to define and display [Route](crate::Route)s.
    children: Children,
) -> impl IntoView {
    // create a new RouterContext and provide it to every component beneath the router
    let router = RouterContext::new(cx, base, fallback);
    provide_context(cx, router);
    provide_context(cx, GlobalSuspenseContext::new(cx));
    if let Some(set_is_routing) = set_is_routing {
        provide_context(cx, SetIsRouting(set_is_routing));
    }

    children(cx)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct SetIsRouting(pub SignalSetter<bool>);

/// Context type that contains information about the current router state.
#[derive(Debug, Clone)]
pub struct RouterContext {
    pub(crate) inner: Rc<RouterContextInner>,
}
pub(crate) struct RouterContextInner {
    pub location: Location,
    pub base: RouteContext,
    pub possible_routes: RefCell<Option<Vec<Branch>>>,
    #[allow(unused)] // used in CSR/hydrate
    base_path: String,
    history: Box<dyn History>,
    cx: Scope,
    reference: ReadSignal<String>,
    set_reference: WriteSignal<String>,
    referrers: Rc<RefCell<Vec<LocationChange>>>,
    state: ReadSignal<State>,
    set_state: WriteSignal<State>,
    pub(crate) is_back: RwSignal<bool>,
    pub(crate) path_stack: StoredValue<Vec<String>>,
}

impl std::fmt::Debug for RouterContextInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RouterContextInner")
            .field("location", &self.location)
            .field("base", &self.base)
            .field("cx", &self.cx)
            .field("reference", &self.reference)
            .field("set_reference", &self.set_reference)
            .field("referrers", &self.referrers)
            .field("state", &self.state)
            .field("set_state", &self.set_state)
            .field("path_stack", &self.path_stack)
            .finish()
    }
}

impl RouterContext {
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        tracing::instrument(level = "trace", skip_all,)
    )]
    pub(crate) fn new(
        cx: Scope,
        base: Option<&'static str>,
        fallback: Option<fn(Scope) -> View>,
    ) -> Self {
        cfg_if! {
            if #[cfg(any(feature = "csr", feature = "hydrate"))] {
                let history = use_context::<RouterIntegrationContext>(cx)
                    .unwrap_or_else(|| RouterIntegrationContext(Rc::new(crate::BrowserIntegration {})));
            } else {
                let history = use_context::<RouterIntegrationContext>(cx).unwrap_or_else(|| {
                    let msg = "No router integration found.\n\nIf you are using this in the browser, \
                        you should enable `features = [\"csr\"]` or `features = [\"hydrate\"] in your \
                        `leptos_router` import.\n\nIf you are using this on the server without a \
                        Leptos server integration, you must call provide_context::<RouterIntegrationContext>(cx, ...) \
                        somewhere above the <Router/>.";
                    leptos::debug_warn!("{}", msg);
                    panic!("{}", msg);
                });
            }
        };

        // Any `History` type gives a way to get a reactive signal of the current location
        // in the browser context, this is drawn from the `popstate` event
        // different server adapters can provide different `History` implementations to allow server routing
        let source = history.location(cx);

        // if initial route is empty, redirect to base path, if it exists
        let base = base.unwrap_or_default();
        let base_path = resolve_path("", base, None);

        if let Some(base_path) = &base_path {
            if source.with_untracked(|s| s.value.is_empty()) {
                history.navigate(&LocationChange {
                    value: base_path.to_string(),
                    replace: true,
                    scroll: false,
                    state: State(None),
                });
            }
        }

        // the current URL
        let (reference, set_reference) =
            create_signal(cx, source.with_untracked(|s| s.value.clone()));

        // the current History.state
        let (state, set_state) =
            create_signal(cx, source.with_untracked(|s| s.state.clone()));

        // we'll use this transition to wait for async resources to load when navigating to a new route
        #[cfg(feature = "transition")]
        let transition = use_transition(cx);

        // Each field of `location` reactively represents a different part of the current location
        let location = create_location(cx, reference, state);
        let referrers: Rc<RefCell<Vec<LocationChange>>> =
            Rc::new(RefCell::new(Vec::new()));

        // Create base route with fallback element
        let base_path = base_path.unwrap_or_default();
        let base = RouteContext::base(cx, &base_path, fallback);

        // Every time the History gives us a new location,
        // 1) start a transition
        // 2) update the reference (URL)
        // 3) update the state
        // this will trigger the new route match below

        create_render_effect(cx, move |_| {
            let LocationChange { value, state, .. } = source.get();
            cx.untrack(move || {
                if value != reference.get() {
                    set_reference.update(move |r| *r = value);
                    set_state.update(move |s| *s = state);
                }
            });
        });

        let inner = Rc::new(RouterContextInner {
            base_path: base_path.into_owned(),
            path_stack: store_value(
                cx,
                vec![location.pathname.get_untracked()],
            ),
            location,
            base,
            history: Box::new(history),
            cx,
            reference,
            set_reference,
            referrers,
            state,
            set_state,
            possible_routes: Default::default(),
            is_back: create_rw_signal(cx, false),
        });

        // handle all click events on anchor tags
        #[cfg(not(feature = "ssr"))]
        leptos::window_event_listener_untyped("click", {
            let inner = Rc::clone(&inner);
            move |ev| inner.clone().handle_anchor_click(ev)
        });
        // TODO on_cleanup remove event listener

        Self { inner }
    }

    /// The current [`pathname`](https://developer.mozilla.org/en-US/docs/Web/API/Location/pathname).
    pub fn pathname(&self) -> Memo<String> {
        self.inner.location.pathname
    }

    /// The [RouteContext] of the base route.
    pub fn base(&self) -> RouteContext {
        self.inner.base.clone()
    }

    /// A list of all possible routes this router can match.
    pub fn possible_branches(&self) -> Vec<Branch> {
        self.inner
            .possible_routes
            .borrow()
            .clone()
            .unwrap_or_default()
    }
}

impl RouterContextInner {
    #[cfg_attr(
        any(debug_assertions, feature = "ssr"),
        tracing::instrument(level = "trace", skip_all,)
    )]
    pub(crate) fn navigate_from_route(
        self: Rc<Self>,
        to: &str,
        options: &NavigateOptions,
    ) -> Result<(), NavigationError> {
        let cx = self.cx;
        let this = Rc::clone(&self);

        cx.untrack(move || {
            let resolved_to = if options.resolve {
                this.base.resolve_path(to)
            } else {
                resolve_path("", to, None).map(String::from)
            };

            // reset count of pending resources at global level
            expect_context::<GlobalSuspenseContext>(cx).reset(cx);

            match resolved_to {
                None => Err(NavigationError::NotRoutable(to.to_string())),
                Some(resolved_to) => {
                    if self.referrers.borrow().len() > 32 {
                        return Err(NavigationError::MaxRedirects);
                    }

                    if resolved_to != this.reference.get()
                        || options.state != (this.state).get()
                    {
                        {
                            self.referrers.borrow_mut().push(LocationChange {
                                value: self.reference.get(),
                                replace: options.replace,
                                scroll: options.scroll,
                                state: self.state.get(),
                            });
                        }
                        let len = self.referrers.borrow().len();

                        let set_reference = self.set_reference;
                        let set_state = self.set_state;
                        let referrers = self.referrers.clone();
                        let this = Rc::clone(&self);

                        let resolved = resolved_to.to_string();
                        let state = options.state.clone();
                        set_reference.update(move |r| *r = resolved);

                        set_state.update({
                            let next_state = state.clone();
                            move |state| *state = next_state
                        });

                        let global_suspense =
                            expect_context::<GlobalSuspenseContext>(cx);
                        let path_stack = self.path_stack;
                        let is_navigating_back = self.is_back.get_untracked();
                        if !is_navigating_back {
                            path_stack.update_value(|stack| {
                                stack.push(resolved_to.clone())
                            });
                        }

                        let set_is_routing = use_context::<SetIsRouting>(cx);
                        if let Some(set_is_routing) = set_is_routing {
                            set_is_routing.0.set(true);
                        }
                        spawn_local(async move {
                            if let Some(set_is_routing) = set_is_routing {
                                global_suspense
                                    .with_inner(|s| s.to_future(cx))
                                    .await;
                                set_is_routing.0.set(false);
                            }

                            if referrers.borrow().len() == len {
                                this.navigate_end(LocationChange {
                                    value: resolved_to,
                                    replace: false,
                                    scroll: true,
                                    state,
                                });
                            }
                        });
                    }

                    Ok(())
                }
            }
        })
    }

    pub(crate) fn navigate_end(self: Rc<Self>, mut next: LocationChange) {
        let first = self.referrers.borrow().get(0).cloned();
        if let Some(first) = first {
            if next.value != first.value || next.state != first.state {
                next.replace = first.replace;
                next.scroll = first.scroll;
                self.history.navigate(&next);
            }
            self.referrers.borrow_mut().clear();
        }
    }

    #[cfg(not(feature = "ssr"))]
    pub(crate) fn handle_anchor_click(self: Rc<Self>, ev: web_sys::Event) {
        use wasm_bindgen::JsValue;

        let ev = ev.unchecked_into::<web_sys::MouseEvent>();
        if ev.default_prevented()
            || ev.button() != 0
            || ev.meta_key()
            || ev.alt_key()
            || ev.ctrl_key()
            || ev.shift_key()
        {
            return;
        }

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
            // TODO "state" is set as a prop, not an attribute
            if !target.is_empty()
                || (href.is_empty() && !a.has_attribute("state"))
            {
                return;
            }

            let rel = a.get_attribute("rel").unwrap_or_default();
            let mut rel = rel.split([' ', '\t']);

            // let browser handle event if it has rel=external or download
            if a.has_attribute("download") || rel.any(|p| p == "external") {
                return;
            }

            let url = Url::try_from(href.as_str()).unwrap();
            let path_name = unescape(&url.pathname);

            // let browser handle this event if it leaves our domain
            // or our base path
            if url.origin
                != leptos_dom::helpers::location().origin().unwrap_or_default()
                || (!self.base_path.is_empty()
                    && !path_name.is_empty()
                    && !path_name
                        .to_lowercase()
                        .starts_with(&self.base_path.to_lowercase()))
            {
                return;
            }

            let to = path_name
                + if url.search.is_empty() { "" } else { "?" }
                + &unescape(&url.search)
                + &unescape(&url.hash);
            let state =
                leptos_dom::helpers::get_property(a.unchecked_ref(), "state")
                    .ok()
                    .and_then(|value| {
                        if value == JsValue::UNDEFINED {
                            None
                        } else {
                            Some(value)
                        }
                    });

            ev.prevent_default();

            let replace =
                leptos_dom::helpers::get_property(a.unchecked_ref(), "replace")
                    .ok()
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false);
            if let Err(e) = self.navigate_from_route(
                &to,
                &NavigateOptions {
                    resolve: false,
                    replace,
                    scroll: !a.has_attribute("noscroll"),
                    state: State(state),
                },
            ) {
                leptos::error!("{e:#?}");
            }
        }
    }
}

/// An error that occurs during navigation.
#[derive(Debug, Error)]
pub enum NavigationError {
    /// The given path is not routable.
    #[error("Path {0:?} is not routable")]
    NotRoutable(String),
    /// Too many redirects occurred during routing (prevents and infinite loop.)
    #[error("Too many redirects")]
    MaxRedirects,
}

/// Options that can be used to configure a navigation. Used with [use_navigate](crate::use_navigate).
#[derive(Clone, Debug)]
pub struct NavigateOptions {
    /// Whether the URL being navigated to should be resolved relative to the current route.
    pub resolve: bool,
    /// If `true` the new location will replace the current route in the history stack, meaning
    /// the "back" button will skip over the current route. (Defaults to `false`).
    pub replace: bool,
    /// If `true`, the router will scroll to the top of the window at the end of navigation.
    /// Defaults to `true.
    pub scroll: bool,
    /// [State](https://developer.mozilla.org/en-US/docs/Web/API/History/state) that should be pushed
    /// onto the history stack during navigation.
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
