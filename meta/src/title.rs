use crate::{use_head, MetaContext, ServerMetaContext};
use leptos::{
    attr::{any_attribute::AnyAttribute, Attribute},
    component,
    oco::Oco,
    prelude::{ArcTrigger, Notify, Track},
    reactive::{effect::RenderEffect, owner::use_context},
    tachys::{
        dom::document,
        hydration::Cursor,
        view::{
            add_attr::AddAnyAttr, Mountable, Position, PositionState, Render,
            RenderHtml,
        },
    },
    text_prop::TextProp,
    IntoView,
};
use or_poisoned::OrPoisoned;
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex, RwLock,
};

/// Contains the current state of the document's `<title>`.
#[derive(Clone, Default)]
pub struct TitleContext {
    id: Arc<AtomicU32>,
    formatter_stack: Arc<RwLock<Vec<(TitleId, Formatter)>>>,
    text_stack: Arc<RwLock<Vec<(TitleId, TextProp)>>>,
    revalidate: ArcTrigger,
    #[allow(clippy::type_complexity)]
    effect: Arc<Mutex<Option<RenderEffect<Option<Oco<'static, str>>>>>>,
}

impl core::fmt::Debug for TitleContext {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("TitleContext").finish()
    }
}

type TitleId = u32;

impl TitleContext {
    fn next_id(&self) -> TitleId {
        self.id.fetch_add(1, Ordering::Relaxed)
    }

    fn invalidate(&self) {
        self.revalidate.notify();
    }

    fn spawn_effect(&self) {
        let this = self.clone();
        let revalidate = self.revalidate.clone();

        let mut effect_lock = self.effect.lock().or_poisoned();
        if effect_lock.is_none() {
            *effect_lock = Some(RenderEffect::new({
                move |_| {
                    revalidate.track();
                    let text = this.as_string();
                    document().set_title(text.as_deref().unwrap_or_default());
                    text
                }
            }));
        }
    }

    fn push_text_and_formatter(
        &self,
        id: TitleId,
        text: Option<TextProp>,
        formatter: Option<Formatter>,
    ) {
        if let Some(text) = text {
            self.text_stack.write().or_poisoned().push((id, text));
        }
        if let Some(formatter) = formatter {
            self.formatter_stack
                .write()
                .or_poisoned()
                .push((id, formatter));
        }
        self.invalidate();
    }

    fn update_text_and_formatter(
        &self,
        id: TitleId,
        text: Option<TextProp>,
        formatter: Option<Formatter>,
    ) {
        let mut text_stack = self.text_stack.write().or_poisoned();
        let mut formatter_stack = self.formatter_stack.write().or_poisoned();
        let text_pos =
            text_stack.iter().position(|(item_id, _)| *item_id == id);
        let formatter_pos = formatter_stack
            .iter()
            .position(|(item_id, _)| *item_id == id);

        match (text_pos, text) {
            (None, None) => {}
            (Some(old), Some(new)) => {
                text_stack[old].1 = new;
                self.invalidate();
            }
            (Some(old), None) => {
                text_stack.remove(old);
                self.invalidate();
            }
            (None, Some(new)) => {
                text_stack.push((id, new));
                self.invalidate();
            }
        }
        match (formatter_pos, formatter) {
            (None, None) => {}
            (Some(old), Some(new)) => {
                formatter_stack[old].1 = new;
                self.invalidate();
            }
            (Some(old), None) => {
                formatter_stack.remove(old);
                self.invalidate();
            }
            (None, Some(new)) => {
                formatter_stack.push((id, new));
                self.invalidate();
            }
        }
    }

    fn remove_id(&self, id: TitleId) -> (Option<TextProp>, Option<Formatter>) {
        let mut text_stack = self.text_stack.write().or_poisoned();
        let text = text_stack
            .iter()
            .position(|(item_id, _)| *item_id == id)
            .map(|pos| text_stack.remove(pos).1);

        let mut formatter_stack = self.formatter_stack.write().or_poisoned();
        let formatter = formatter_stack
            .iter()
            .position(|(item_id, _)| *item_id == id)
            .map(|pos| formatter_stack.remove(pos).1);

        self.invalidate();

        (text, formatter)
    }

    /// Converts the title into a string that can be used as the text content of a `<title>` tag.
    pub fn as_string(&self) -> Option<Oco<'static, str>> {
        let title = self
            .text_stack
            .read()
            .or_poisoned()
            .last()
            .map(|n| n.1.get());

        title.map(|title| {
            if let Some(formatter) =
                self.formatter_stack.read().or_poisoned().last()
            {
                (formatter.1 .0)(title.into_owned()).into()
            } else {
                title
            }
        })
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
/// use leptos::prelude::*;
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
    let id = meta.title.next_id();
    if let Some(cx) = server_ctx {
        // if we are server rendering, we will not actually use these values via RenderHtml
        // instead, they'll be handled separately by the server integration
        // so it's safe to take them out of the props here
        cx.title
            .push_text_and_formatter(id, text.take(), formatter.take());
    };

    TitleView {
        id,
        meta,
        formatter,
        text,
    }
}

struct TitleView {
    id: u32,
    meta: MetaContext,
    formatter: Option<Formatter>,
    text: Option<TextProp>,
}

struct TitleViewState {
    id: TitleId,
    meta: MetaContext,
    // these are only Some(_) after being unmounted, and hold these values until dropped or remounted
    formatter: Option<Formatter>,
    text: Option<TextProp>,
}

impl Drop for TitleViewState {
    fn drop(&mut self) {
        // when TitleViewState is dropped, it should remove its ID from the text and formatter stacks
        // so that they no longer appear. it will also revalidate the whole title in case this one was active
        self.meta.title.remove_id(self.id);
    }
}

impl Render for TitleView {
    type State = TitleViewState;

    fn build(self) -> Self::State {
        let TitleView {
            id,
            meta,
            formatter,
            text,
        } = self;
        meta.title.spawn_effect();
        TitleViewState {
            id,
            meta,
            text,
            formatter,
        }
    }

    fn rebuild(self, _state: &mut Self::State) {
        self.meta.title.update_text_and_formatter(
            self.id,
            self.text,
            self.formatter,
        );
    }
}

impl AddAnyAttr for TitleView {
    type Output<SomeNewAttr: Attribute> = TitleView;

    fn add_any_attr<NewAttr: Attribute>(
        self,
        _attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        self
    }
}

impl RenderHtml for TitleView {
    type AsyncOutput = Self;
    type Owned = Self;

    const MIN_LENGTH: usize = 0;

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }

    fn to_html_with_buf(
        self,
        _buf: &mut String,
        _position: &mut Position,
        _escape: bool,
        _mark_branches: bool,
        _extra_attrs: Vec<AnyAttribute>,
    ) {
        // meta tags are rendered into the buffer stored into the context
        // the value has already been taken out, when we're on the server
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        _cursor: &Cursor,
        _position: &PositionState,
    ) -> Self::State {
        let TitleView {
            id,
            meta,
            formatter,
            text,
        } = self;
        meta.title.spawn_effect();
        // these need to be pushed here, rather than on mount, because mount() is not called when hydrating
        meta.title.push_text_and_formatter(id, text, formatter);
        TitleViewState {
            id,
            meta,
            text: None,
            formatter: None,
        }
    }

    fn into_owned(self) -> Self::Owned {
        self
    }
}

impl Mountable for TitleViewState {
    fn unmount(&mut self) {
        let (text, formatter) = self.meta.title.remove_id(self.id);
        if text.is_some() {
            self.text = text;
        }
        if formatter.is_some() {
            self.formatter = formatter;
        }
    }

    fn mount(
        &mut self,
        _parent: &leptos::tachys::renderer::types::Element,
        _marker: Option<&leptos::tachys::renderer::types::Node>,
    ) {
        // TitleView::el() guarantees that there is a <title> in the <head>
        // so there is no element to be mounted
        //
        // "mounting" in this case means that we actually want this title to be in active use
        // as a result, we will push it into the title stack and revalidate
        self.meta.title.push_text_and_formatter(
            self.id,
            self.text.take(),
            self.formatter.take(),
        );
    }

    fn insert_before_this(&self, _child: &mut dyn Mountable) -> bool {
        false
    }

    fn elements(&self) -> Vec<leptos::tachys::renderer::types::Element> {
        vec![]
    }
}
