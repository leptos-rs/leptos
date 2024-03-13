use crate::ServerMetaContext;
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
use web_sys::HtmlElement;

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
    /// Arbitrary attributes to add to the `<body>`.
    #[prop(attrs)]
    mut attributes: Vec<AnyAttribute<Dom>>,
) -> impl IntoView {
    if let Some(meta) = use_context::<ServerMetaContext>() {
        let mut meta = meta.inner.write().or_poisoned();
        // if we are server rendering, we will not actually use these values via RenderHtml
        // instead, they'll be handled separately by the server integration
        // so it's safe to take them out of the props here
        meta.body = mem::take(&mut attributes);
    }

    BodyView { attributes }
}

struct BodyView {
    attributes: Vec<AnyAttribute<Dom>>,
}

struct BodyViewState {
    el: HtmlElement,
    attributes: Vec<AnyAttributeState<Dom>>,
}

impl Render<Dom> for BodyView {
    type State = BodyViewState;
    type FallibleState = BodyViewState;

    fn build(self) -> Self::State {
        let el = document().body().expect("there to be a <body> element");

        let attributes = self
            .attributes
            .into_iter()
            .map(|attr| attr.build(&el))
            .collect();

        BodyViewState { el, attributes }
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

        let attributes = self
            .attributes
            .into_iter()
            .map(|attr| attr.hydrate::<FROM_SERVER>(&el))
            .collect();

        BodyViewState { el, attributes }
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
