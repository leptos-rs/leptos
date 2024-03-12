use crate::{use_head, ServerMetaContext};
use indexmap::IndexMap;
use leptos::{
    component,
    oco::Oco,
    reactive_graph::{effect::RenderEffect, owner::use_context},
    tachys::{
        dom::document,
        error::Result,
        html::attribute::{
            any_attribute::{AnyAttribute, AnyAttributeState},
            Attribute,
        },
        hydration::Cursor,
        reactive_graph::RenderEffectState,
        renderer::{dom::Dom, Renderer},
        view::{Mountable, Position, PositionState, Render, RenderHtml},
    },
    text_prop::TextProp,
    IntoView,
};
use or_poisoned::OrPoisoned;
use std::{
    cell::RefCell,
    collections::HashMap,
    mem,
    rc::Rc,
    sync::{Arc, RwLock},
};
use web_sys::{HtmlBodyElement, HtmlElement};

/// Contains the current metadata for the document's `<body>`.
#[derive(Clone, Default)]
pub struct BodyContext {
    class: Arc<RwLock<Option<TextProp>>>,
    attributes: Arc<RwLock<Vec<AnyAttribute<Dom>>>>,
}

impl BodyContext {
    /// Converts the `<body>` metadata into an HTML string.
    ///
    /// This consumes the list of `attributes`, and should only be called once per request.
    pub fn to_string(&self) -> Option<String> {
        let mut buf = String::from(" ");
        if let Some(class) = &*self.class.read().or_poisoned() {
            buf.push_str("class=\"");
            buf.push_str(&class.get());
            buf.push_str("\" ");
        };

        let attributes = mem::take(&mut *self.attributes.write().or_poisoned());

        for attr in attributes {
            attr.to_html(
                &mut buf,
                &mut String::new(),
                &mut String::new(),
                &mut String::new(),
            );
            buf.push(' ');
        }

        if buf.trim().is_empty() {
            None
        } else {
            Some(buf)
        }
    }
}

impl core::fmt::Debug for BodyContext {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BodyContext").finish_non_exhaustive()
    }
}

/// A component to set metadata on the document’s `<body>` element from
/// within the application.
///
/// ```
/// use leptos::*;
/// use leptos_meta::*;
///
/// #[component]
/// fn MyApp() -> impl IntoView {
///     provide_meta_context();
///     let (prefers_dark, set_prefers_dark) = create_signal(false);
///     let body_class = move || {
///         if prefers_dark.get() {
///             "dark".to_string()
///         } else {
///             "light".to_string()
///         }
///     };
///
///     view! {
///       <main>
///         <Body class=body_class attr:class="foo"/>
///       </main>
///     }
/// }
/// ```
#[component]
pub fn Body(
    /// The `class` attribute on the `<body>`.
    #[prop(optional, into)]
    class: Option<TextProp>,
    /// Arbitrary attributes to add to the `<body>`
    #[prop(attrs)]
    mut attributes: Vec<AnyAttribute<Dom>>,
) -> impl IntoView {
    if let Some(meta) = use_context::<ServerMetaContext>() {
        *meta.body.class.write().or_poisoned() = class.clone();

        // these can safely be taken out if the server context is present
        // server rendering is handled separately, not via RenderHtml
        *meta.body.attributes.write().or_poisoned() = mem::take(&mut attributes)
    }

    BodyView { class, attributes }
}

struct BodyView {
    class: Option<TextProp>,
    attributes: Vec<AnyAttribute<Dom>>,
}

struct BodyViewState {
    el: HtmlElement,
    class: Option<RenderEffect<Oco<'static, str>>>,
    attributes: Vec<AnyAttributeState<Dom>>,
}

impl Render<Dom> for BodyView {
    type State = BodyViewState;
    type FallibleState = BodyViewState;

    fn build(self) -> Self::State {
        let el = document().body().expect("there to be a <body> element");
        let class = self.class.map(|class| {
            RenderEffect::new({
                let el = el.clone();
                move |prev| {
                    let next = class.get();
                    if prev.as_ref() != Some(&next) {
                        if let Err(e) = el.set_attribute("class", &next) {
                            web_sys::console::error_1(&e);
                        }
                    }
                    next
                }
            })
        });

        let attributes = self
            .attributes
            .into_iter()
            .map(|attr| attr.build(&el))
            .collect();

        BodyViewState {
            el,
            class,
            attributes,
        }
    }

    fn rebuild(self, state: &mut Self::State) {
        // TODO rebuilding dynamic things like this
    }

    fn try_build(self) -> Result<Self::FallibleState> {
        Ok(self.build())
    }

    fn try_rebuild(self, state: &mut Self::FallibleState) -> Result<()> {
        self.rebuild(state);
        Ok(())
    }
}

impl RenderHtml<Dom> for BodyView {
    const MIN_LENGTH: usize = 0;

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {}

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<Dom>,
        position: &PositionState,
    ) -> Self::State {
        let el = document().body().expect("there to be a <body> element");
        let class = self.class.map(|class| {
            RenderEffect::new({
                let el = el.clone();
                move |prev| {
                    let next = class.get();
                    if prev.is_none() {
                        return next;
                    }

                    if prev.as_ref() != Some(&next) {
                        if let Err(e) = el.set_attribute("class", &next) {
                            web_sys::console::error_1(&e);
                        }
                    }
                    next
                }
            })
        });

        let attributes = self
            .attributes
            .into_iter()
            .map(|attr| attr.hydrate::<FROM_SERVER>(&el))
            .collect();

        BodyViewState {
            el,
            class,
            attributes,
        }
    }
}

impl Mountable<Dom> for BodyViewState {
    fn unmount(&mut self) {}

    fn mount(
        &mut self,
        parent: &<Dom as Renderer>::Element,
        marker: Option<&<Dom as Renderer>::Node>,
    ) {
    }

    fn insert_before_this(
        &self,
        parent: &<Dom as Renderer>::Element,
        child: &mut dyn Mountable<Dom>,
    ) -> bool {
        true
    }
}
