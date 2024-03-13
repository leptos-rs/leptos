use crate::{use_head, MetaContext, ServerMetaContext};
use leptos::{
    component,
    oco::Oco,
    reactive_graph::{
        effect::RenderEffect,
        owner::{use_context, Owner},
    },
    tachys::{
        dom::document,
        error::Result,
        hydration::Cursor,
        renderer::{dom::Dom, Renderer},
        view::{Mountable, PositionState, Render, RenderHtml},
    },
    text_prop::TextProp,
    IntoView,
};
use or_poisoned::OrPoisoned;
use send_wrapper::SendWrapper;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, RwLock},
};
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::{Element, HtmlTitleElement};

/// Contains the current state of the document's `<title>`.
#[derive(Clone, Default)]
pub struct TitleContext {
    el: Arc<RwLock<Option<SendWrapper<HtmlTitleElement>>>>,
    formatter: Arc<RwLock<Option<Formatter>>>,
    text: Arc<RwLock<Option<TextProp>>>,
}

impl TitleContext {
    /// Converts the title into a string that can be used as the text content of a `<title>` tag.
    pub fn as_string(&self) -> Option<Oco<'static, str>> {
        let title = self.text.read().or_poisoned().as_ref().map(TextProp::get);
        title.map(|title| {
            if let Some(formatter) = &*self.formatter.read().or_poisoned() {
                (formatter.0)(title.into_owned()).into()
            } else {
                title
            }
        })
    }
}

impl core::fmt::Debug for TitleContext {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("TitleContext").finish()
    }
}

/// A function that is applied to the text value before setting `document.title`.
#[repr(transparent)]
pub struct Formatter(Box<dyn Fn(String) -> String + Send + Sync>);

impl<F> From<F> for Formatter
where
    F: Fn(String) -> String + Send + Sync + 'static,
{
    #[inline(always)]
    fn from(f: F) -> Formatter {
        Formatter(Box::new(f))
    }
}

/// A component to set the document’s title by creating an [`HTMLTitleElement`](https://developer.mozilla.org/en-US/docs/Web/API/HTMLTitleElement).
///
/// The `title` and `formatter` can be set independently of one another. For example, you can create a root-level
/// `<Title formatter=.../>` that will wrap each of the text values of `<Title/>` components created lower in the tree.
///
/// ```
/// use leptos::*;
/// use leptos_meta::*;
///
/// #[component]
/// fn MyApp() -> impl IntoView {
///     provide_meta_context();
///     let formatter = |text| format!("{text} — Leptos Online");
///
///     view! {
///       <main>
///         <Title formatter/>
///         // ... routing logic here
///       </main>
///     }
/// }
///
/// #[component]
/// fn PageA() -> impl IntoView {
///     view! {
///       <main>
///         <Title text="Page A"/> // sets title to "Page A — Leptos Online"
///       </main>
///     }
/// }
///
/// #[component]
/// fn PageB() -> impl IntoView {
///     view! {
///       <main>
///         <Title text="Page B"/> // sets title to "Page B — Leptos Online"
///       </main>
///     }
/// }
/// ```
#[component]
pub fn Title(
    /// A function that will be applied to any text value before it’s set as the title.
    #[prop(optional, into)]
    mut formatter: Option<Formatter>,
    /// Sets the current `document.title`.
    #[prop(optional, into)]
    mut text: Option<TextProp>,
) -> impl IntoView {
    let meta = use_head();
    let server_ctx = use_context::<ServerMetaContext>();
    if let Some(cx) = server_ctx {
        // if we are server rendering, we will not actually use these values via RenderHtml
        // instead, they'll be handled separately by the server integration
        // so it's safe to take them out of the props here
        if let Some(formatter) = formatter.take() {
            *cx.title.formatter.write().or_poisoned() = Some(formatter);
        }
        if let Some(text) = text.take() {
            *cx.title.text.write().or_poisoned() = Some(text);
        }
    };

    TitleView {
        meta,
        formatter,
        text,
    }
}

#[derive(Debug)]
struct TitleView {
    meta: MetaContext,
    formatter: Option<Formatter>,
    text: Option<TextProp>,
}

impl TitleView {
    fn el(&self) -> Element {
        let mut el_ref = self.meta.title.el.write().or_poisoned();
        let el = if let Some(el) = &*el_ref {
            el.clone()
        } else {
            match document().query_selector("title") {
                Ok(Some(title)) => title.unchecked_into(),
                _ => {
                    let el_ref = meta.title.el.clone();
                    let el = document().create_element("title").unwrap_throw();
                    let head = document().head().unwrap_throw();
                    head.append_child(el.unchecked_ref()).unwrap_throw();

                    Owner::on_cleanup({
                        let el = el.clone();
                        move || {
                            _ = head.remove_child(&el);
                            *el_ref.borrow_mut() = None;
                        }
                    });

                    el.unchecked_into()
                }
            }
        };
        *el_ref = Some(el.clone().unchecked_into());

        el
    }
}

#[derive(Debug)]
struct TitleViewState {
    el: Element,
    formatter: Option<Formatter>,
    text: Option<TextProp>,
    effect: RenderEffect<Oco<'static, str>>,
}

impl Render<Dom> for TitleView {
    type State = TitleViewState;
    type FallibleState = TitleViewState;

    fn build(self) -> Self::State {
        let el = self.el();
        let meta = self.meta;
        let effect = RenderEffect::new(move |prev| {
            let text = meta.title.as_string().unwrap_or_default();

            if prev.as_ref() != Some(&text) {
                el.set_text_content(Some(&text));
            }

            text
        });
        TitleViewState {
            el,
            formatter: self.formatter,
            text: self.text,
            effect,
        }
    }

    fn rebuild(self, state: &mut Self::State) {
        // TODO  should this rebuild?
    }

    fn try_build(self) -> Result<Self::FallibleState> {
        Ok(self.build())
    }

    fn try_rebuild(self, state: &mut Self::FallibleState) -> Result<()> {
        self.rebuild(state);
        Ok(())
    }
}

impl RenderHtml<Dom> for TitleView {
    const MIN_LENGTH: usize = 0;

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut leptos::tachys::view::Position,
    ) {
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<Dom>,
        position: &PositionState,
    ) -> Self::State {
        self.build()
    }
}

impl Mountable<Dom> for TitleViewState {
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
