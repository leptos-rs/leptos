use super::{
    Mountable, Position, PositionState, Render, RenderHtml, ToTemplate,
};
use crate::{dom::document, hydration::Cursor, renderer::dom::Dom};
use once_cell::unsync::Lazy;
use rustc_hash::FxHashMap;
use std::{any::TypeId, cell::RefCell};
use wasm_bindgen::JsCast;
use web_sys::HtmlTemplateElement;

thread_local! {
    static TEMPLATE_ELEMENT: Lazy<HtmlTemplateElement> =
        Lazy::new(|| document().create_element("template").unwrap().unchecked_into());
}

pub struct ViewTemplate<V: Render<Dom> + ToTemplate> {
    view: V,
}

thread_local! {
    static TEMPLATES: RefCell<FxHashMap<TypeId, HtmlTemplateElement>> = Default::default();
}

impl<V: Render<Dom> + ToTemplate + 'static> ViewTemplate<V> {
    pub fn new(view: V) -> Self {
        Self { view }
    }

    fn to_template() -> HtmlTemplateElement {
        TEMPLATES.with(|t| {
            t.borrow_mut()
                .entry(TypeId::of::<V>())
                .or_insert_with(|| {
                    let tpl = TEMPLATE_ELEMENT.with(|t| {
                        t.clone_node()
                            .unwrap()
                            .unchecked_into::<HtmlTemplateElement>()
                    });
                    /* let mut buf = String::new();
                    let mut class = String::new();
                    let mut style = String::new();
                    V::to_template(
                        &mut buf,
                        &mut class,
                        &mut style,
                        &mut Default::default(),
                    );
                    tpl.set_inner_html(&buf); */
                    //log(&format!("setting template to {:?}", V::TEMPLATE));
                    tpl.set_inner_html(V::TEMPLATE);
                    tpl
                })
                .clone()
        })
    }
}

impl<V> Render<Dom> for ViewTemplate<V>
where
    V: Render<Dom> + RenderHtml<Dom> + ToTemplate + 'static,
    V::State: Mountable<Dom>,
{
    type State = V::State;

    fn build(self) -> Self::State {
        let tpl = Self::to_template();
        let contents = tpl.content().clone_node_with_deep(true).unwrap();
        self.view.hydrate::<false>(
            &Cursor::new(contents.unchecked_into()),
            &Default::default(),
        )
    }

    fn rebuild(self, state: &mut Self::State) {
        self.view.rebuild(state)
    }
}

impl<V> RenderHtml<Dom> for ViewTemplate<V>
where
    V: RenderHtml<Dom> + ToTemplate + 'static,
    V::State: Mountable<Dom>,
{
    const MIN_LENGTH: usize = V::MIN_LENGTH;

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        self.view.to_html_with_buf(buf, position)
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<Dom>,
        position: &PositionState,
    ) -> Self::State {
        self.view.hydrate::<FROM_SERVER>(cursor, position)
    }
}

impl<V> ToTemplate for ViewTemplate<V>
where
    V: RenderHtml<Dom> + ToTemplate + 'static,
    V::State: Mountable<Dom>,
{
    const TEMPLATE: &'static str = V::TEMPLATE;

    fn to_template(
        buf: &mut String,
        class: &mut String,
        style: &mut String,
        inner_html: &mut String,
        position: &mut Position,
    ) {
        V::to_template(buf, class, style, inner_html, position);
    }
}
