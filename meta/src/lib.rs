use std::{cell::RefCell, fmt::Debug, rc::Rc};

use leptos::*;

#[derive(Debug, Clone, Default)]
pub struct MetaContext {
    title: TitleContext,
}

pub fn use_head(cx: Scope) -> MetaContext {
    match use_context::<MetaContext>(cx) {
        None => {
            log::warn!("use_head() can only be called if a MetaContext has been provided");
            panic!()
        }
        Some(ctx) => ctx,
    }
}

impl MetaContext {
    pub fn new() -> Self {
        Default::default()
    }
}

#[derive(Clone, Default)]
pub struct TitleContext {
    el: Rc<RefCell<Option<web_sys::HtmlTitleElement>>>,
    formatter: Rc<RefCell<Option<Formatter>>>,
    text: Rc<RefCell<Option<TextProp>>>,
}

impl Debug for TitleContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("TitleContext").finish()
    }
}

pub struct TextProp(Box<dyn Fn() -> String>);

impl Debug for TextProp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("TextProp").finish()
    }
}

impl From<String> for TextProp {
    fn from(s: String) -> Self {
        TextProp(Box::new(move || s.clone()))
    }
}

impl From<&str> for TextProp {
    fn from(s: &str) -> Self {
        let s = s.to_string();
        TextProp(Box::new(move || s.clone()))
    }
}

impl<F> From<F> for TextProp
where
    F: Fn() -> String + 'static,
{
    fn from(s: F) -> Self {
        TextProp(Box::new(s))
    }
}

pub struct Formatter(Box<dyn Fn(String) -> String>);

impl<F> From<F> for Formatter
where
    F: Fn(String) -> String + 'static,
{
    fn from(f: F) -> Formatter {
        Formatter(Box::new(f))
    }
}

#[component]
pub fn Title(cx: Scope, formatter: Option<Formatter>, text: Option<TextProp>) {
    let meta = use_head(cx);
    if let Some(formatter) = formatter {
        *meta.title.formatter.borrow_mut() = Some(formatter);
    }
    if let Some(text) = text {
        *meta.title.text.borrow_mut() = Some(text.into());
    }

    let el_ref = meta.title.el.borrow_mut();
    let el = if let Some(el) = &*el_ref {
        el.clone()
    } else {
        match document().query_selector("title") {
            Ok(Some(title)) => title.unchecked_into(),
            _ => {
                let el = document().create_element("title").unwrap_throw();
                document()
                    .query_selector("head")
                    .unwrap_throw()
                    .unwrap_throw()
                    .append_child(el.unchecked_ref())
                    .unwrap_throw();
                el.unchecked_into()
            }
        }
    };
    create_render_effect(cx, move |_| {
        let text = meta
            .title
            .text
            .borrow()
            .as_ref()
            .map(|f| (f.0)())
            .unwrap_or_default();
        let text = if let Some(formatter) = &*meta.title.formatter.borrow() {
            (formatter.0)(text)
        } else {
            text
        };

        el.set_text_content(Some(&text));
    });
}
