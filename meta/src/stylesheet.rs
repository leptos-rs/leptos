use crate::{use_head, MetaContext};
use leptos::*;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[derive(Clone, Default, Debug)]
pub struct StylesheetContext {
    els: Rc<RefCell<HashMap<String, Option<web_sys::HtmlLinkElement>>>>,
}

impl StylesheetContext {
    pub fn as_string(&self) -> String {
        self.els
            .borrow()
            .iter()
            .map(|(href, _)| format!(r#"<link rel="stylesheet" href="{href}">"#))
            .collect()
    }
}

#[cfg(feature = "ssr")]
#[component]
pub fn Stylesheet(cx: Scope, href: String) {
    let meta = use_head(cx);
    meta.stylesheets.els.borrow_mut().insert(href, None);
}

#[cfg(not(feature = "ssr"))]
#[component]
pub fn Stylesheet(cx: Scope, href: String) {
    use leptos::document;

    let meta = use_head(cx);

    // TODO I guess this will create a duplicated <link> when hydrating
    let existing_el = {
        let els = meta.stylesheets.els.borrow();
        els.get(&href).cloned()
    };
    if let Some(Some(_)) = existing_el {
        log::warn!("<Stylesheet/> already loaded stylesheet {href}");
    } else {
        let el = document().create_element("link").unwrap_throw();
        el.set_attribute("rel", "stylesheet").unwrap_throw();
        el.set_attribute("href", &href).unwrap_throw();
        document()
            .query_selector("head")
            .unwrap_throw()
            .unwrap_throw()
            .append_child(el.unchecked_ref())
            .unwrap_throw();
        meta.stylesheets
            .els
            .borrow_mut()
            .insert(href, Some(el.unchecked_into()));
    }
}
