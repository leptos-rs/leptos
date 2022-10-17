use crate::{use_head, TextProp};
use leptos::*;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Default)]
pub struct TitleContext {
    #[cfg(not(feature = "ssr"))]
    el: Rc<RefCell<Option<web_sys::HtmlTitleElement>>>,
    formatter: Rc<RefCell<Option<Formatter>>>,
    text: Rc<RefCell<Option<TextProp>>>,
}

impl TitleContext {
    pub fn as_string(&self) -> Option<String> {
        let title = self.text.borrow().as_ref().map(|f| (f.0)());
        title.map(|title| {
            if let Some(formatter) = &*self.formatter.borrow() {
                (formatter.0)(title)
            } else {
                title
            }
        })
    }
}

impl std::fmt::Debug for TitleContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("TitleContext").finish()
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

#[cfg(feature = "ssr")]
#[component]
pub fn Title(cx: Scope, formatter: Option<Formatter>, text: Option<TextProp>) {
    let meta = use_head(cx);
    if let Some(formatter) = formatter {
        *meta.title.formatter.borrow_mut() = Some(formatter);
    }
    if let Some(text) = text {
        *meta.title.text.borrow_mut() = Some(text.into());
    }
    log::debug!("setting title to {:?}", meta.title.as_string());
}

#[cfg(not(feature = "ssr"))]
#[component]
pub fn Title(cx: Scope, formatter: Option<Formatter>, text: Option<TextProp>) {
    use crate::use_head;

    let meta = use_head(cx);
    if let Some(formatter) = formatter {
        *meta.title.formatter.borrow_mut() = Some(formatter);
    }
    if let Some(text) = text {
        *meta.title.text.borrow_mut() = Some(text.into());
    }

    let el = {
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
        el
    };

    create_render_effect(cx, move |_| {
        let text = meta.title.as_string().unwrap_or_default();

        el.set_text_content(Some(&text));
    });
}
