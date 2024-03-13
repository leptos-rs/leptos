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
use web_sys::{Element, HtmlElement};

/// A component to set metadata on the document’s `<html>` element from
/// within the application.
///
/// ```
/// use leptos::*;
/// use leptos_meta::*;
///
/// #[component]
/// fn MyApp() -> impl IntoView {
///     provide_meta_context();
///
///     view! {
///       <main>
///         <Html
///           lang="he"
///           dir="rtl"
///           // arbitrary additional attributes can be passed via `attr:`
///           attr:data-theme="dark"
///         />
///       </main>
///     }
/// }
/// ```
#[component]
pub fn Html(
    /// Arbitrary attributes to add to the `<html>`
    #[prop(attrs)]
    mut attributes: Vec<AnyAttribute<Dom>>,
) -> impl IntoView {
    if let Some(meta) = use_context::<ServerMetaContext>() {
        let mut meta = meta.inner.write().or_poisoned();
        // if we are server rendering, we will not actually use these values via RenderHtml
        // instead, they'll be handled separately by the server integration
        // so it's safe to take them out of the props here
        meta.html = mem::take(&mut attributes);
    }

    HtmlView { attributes }
}

struct HtmlView {
    attributes: Vec<AnyAttribute<Dom>>,
}

struct HtmlViewState {
    el: Element,
    attributes: Vec<AnyAttributeState<Dom>>,
}

impl Render<Dom> for HtmlView {
    type State = HtmlViewState;
    type FallibleState = HtmlViewState;

    fn build(self) -> Self::State {
        let el = document()
            .document_element()
            .expect("there to be a <html> element");

        let attributes = self
            .attributes
            .into_iter()
            .map(|attr| attr.build(&el))
            .collect();

        HtmlViewState { el, attributes }
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

impl RenderHtml<Dom> for HtmlView {
    const MIN_LENGTH: usize = 0;

    fn to_html_with_buf(self, _buf: &mut String, _position: &mut Position) {
        // meta tags are rendered into the buffer stored into the context
        // the value has already been taken out, when we're on the server
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        _cursor: &Cursor<Dom>,
        _position: &PositionState,
    ) -> Self::State {
        let el = document()
            .document_element()
            .expect("there to be a <html> element");

        let attributes = self
            .attributes
            .into_iter()
            .map(|attr| attr.hydrate::<FROM_SERVER>(&el))
            .collect();

        HtmlViewState { el, attributes }
    }
}

impl Mountable<Dom> for HtmlViewState {
    fn unmount(&mut self) {}

    fn mount(
        &mut self,
        _parent: &<Dom as Renderer>::Element,
        _marker: Option<&<Dom as Renderer>::Node>,
    ) {
        // <Html> only sets attributes
        // the <html> tag doesn't need to be mounted anywhere, of course
    }

    fn insert_before_this(
        &self,
        _parent: &<Dom as Renderer>::Element,
        _child: &mut dyn Mountable<Dom>,
    ) -> bool {
        true
    }
}
