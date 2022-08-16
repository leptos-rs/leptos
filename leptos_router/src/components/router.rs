use std::{any::Any, cell::RefCell, future::Future};

use leptos_dom::IntoChild;
use leptos_reactive::{ReadSignal, Scope, WriteSignal};
use thiserror::Error;

use crate::{
    create_location, resolve_path, DataFunction, HistoryIntegration, Integration, Location,
    LocationChange, Params, RouteContext, State,
};

pub struct RouterProps<C, D, Fu, T>
where
    C: IntoChild,
    D: Fn(Params, Location) -> Fu + Clone + 'static,
    Fu: Future<Output = T>,
    T: Any + 'static,
{
    base: Option<String>,
    data: Option<D>,
    children: C,
}

#[allow(non_snake_case)]
pub fn Router<C, D, Fu, T>(cx: Scope, props: RouterProps<C, D, Fu, T>) -> C
where
    C: IntoChild,
    D: Fn(Params, Location) -> Fu + Clone + 'static,
    Fu: Future<Output = T>,
    T: Any + 'static,
{
    let integration = HistoryIntegration {};
    cx.provide_context(RouterContext::new(
        cx,
        integration,
        props.base,
        props.data.map(|data| DataFunction::from(data)),
    ));

    props.children
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouterContext {
    cx: Scope,
    reference: ReadSignal<String>,
    set_reference: WriteSignal<String>,
    referrers: RefCell<Vec<LocationChange>>,
    source: ReadSignal<LocationChange>,
    set_source: WriteSignal<LocationChange>,
    state: ReadSignal<State>,
    set_state: WriteSignal<State>,
}

impl RouterContext {
    pub fn new(
        cx: Scope,
        integration: impl Integration,
        base: Option<String>,
        data: Option<DataFunction>,
    ) -> Self {
        let (source, set_source) = integration.normalize(cx);
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
        let (reference, set_reference) = cx.create_signal(source.with(|s| s.value.clone()));
        let (state, set_state) = cx.create_signal(source.with(|s| s.state.clone()));
        let transition = cx.use_transition();
        let location = create_location(cx, reference, state);
        let referrers: Vec<LocationChange> = Vec::new();

        let base_path = RouteContext::new(&base_path.unwrap_or_default());

        if let Some(data) = data {
            todo!()
        }

        cx.create_render_effect(move |_| {
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

        // TODO handle anchor click

        Self {
            cx,
            reference,
            set_reference,
            referrers: RefCell::new(referrers),
            source,
            set_source,
            state,
            set_state,
        }
    }

    pub fn navigate_from_route(
        &self,
        route: &RouteContext,
        to: &str,
        options: &NavigateOptions,
    ) -> Result<(), NavigationError> {
        self.cx.untrack(move || {
            let resolved_to = if options.resolve {
                route.resolve_path(to)
            } else {
                resolve_path("", to, None)
            };

            match resolved_to {
                None => Err(NavigationError::NotRoutable(to.to_string())),
                Some(resolved_to) => {
                    if self.referrers.borrow().len() > 32 {
                        return Err(NavigationError::MaxRedirects);
                    }

                    let current = self.reference.get();

                    if resolved_to != current || options.state != self.state.get() {
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
                                    state: self.state.get(),
                                });
                            }
                            let len = self.referrers.borrow().len();

                            let transition = self.cx.use_transition();
                            transition.start(move || {
                                self.set_reference.update({
                                    let resolved = resolved_to.to_string();
                                    move |r| *r = resolved
                                });
                                self.set_state.update({
                                    let next_state = options.state.clone();
                                    move |state| *state = next_state
                                });
                                if self.referrers.borrow().len() == len {
                                    self.navigate_end(LocationChange {
                                        value: resolved_to.to_string(),
                                        replace: false,
                                        scroll: true,
                                        state: options.state.clone(),
                                    })
                                }
                            });
                        }
                    }

                    Ok(())
                }
            }
        })
    }

    fn navigate_end(&self, next: LocationChange) {
        let first = self.referrers.borrow().get(0).cloned();
        if let Some(first) = first {
            if next.value != first.value || next.state != first.state {
                self.set_source.update(|source| {
                    *source = next;
                    source.replace = first.replace;
                    source.scroll = first.scroll;
                })
            }
            self.referrers.borrow_mut().clear();
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
