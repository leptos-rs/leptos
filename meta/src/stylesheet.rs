use crate::use_head;
use cfg_if::cfg_if;
use leptos::*;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use typed_builder::TypedBuilder;

/// Manages all of the stylesheets set by [Stylesheet] components.
#[derive(Clone, Default, Debug)]
pub struct StylesheetContext {
    els: Rc<RefCell<HashMap<(Option<String>, String), Option<web_sys::HtmlLinkElement>>>>,
}

impl StylesheetContext {
    /// Converts the set of stylesheets into an HTML string that can be injected into the `<head>`.
    pub fn as_string(&self) -> String {
        self.els
            .borrow()
            .iter()
            .map(|((id, href), _)| {
                if let Some(id) = id {
                    format!(r#"<link rel="stylesheet" id="{id}" href="{href}">"#)
                } else {
                    format!(r#"<link rel="stylesheet" href="{href}">"#)
                }
            })
            .collect()
    }
}

/// Properties for the [Stylesheet] component.
#[derive(TypedBuilder)]
pub struct StylesheetProps {
    /// The URL at which the stylesheet can be located.
    #[builder(setter(into))]
    pub href: String,
    /// The URL at which the stylesheet can be located.
    #[builder(setter(into, strip_option), default = None)]
    pub id: Option<String>,
}

/// Injects an [HTMLLinkElement](https://developer.mozilla.org/en-US/docs/Web/API/HTMLLinkElement) into the document
/// head that loads a stylesheet from the URL given by the `href` property.
///
/// ```
/// use leptos::*;
/// use leptos_meta::*;
///
/// #[component]
/// fn MyApp(cx: Scope) -> Element {
///   provide_context(cx, MetaContext::new());
///
///   view! { cx,
///     <main>
///       <Stylesheet href="/style.css"/>
///     </main>
///   }
/// }
/// ```
#[allow(non_snake_case)]
pub fn Stylesheet(cx: Scope, props: StylesheetProps) {
    let StylesheetProps { href, id } = props;
    cfg_if! {
        if #[cfg(any(feature = "csr", feature = "hydrate"))] {
            use leptos::document;

            let meta = use_head(cx);

            // TODO I guess this will create a duplicated <link> when hydrating
            let existing_el = {
                let els = meta.stylesheets.els.borrow();
                let key = (id.clone(), href.clone());
                els.get(&key).cloned()
            };
            if let Some(Some(_)) = existing_el {
                leptos::leptos_dom::debug_warn!("<Stylesheet/> already loaded stylesheet {href}");
            } else {
                let el = document().create_element("link").unwrap_throw();
                el.set_attribute("rel", "stylesheet").unwrap_throw();
                if let Some(id_val) = &id{
                    el.set_attribute("id", id_val).unwrap_throw();
                }
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
                    .insert((id, href), Some(el.unchecked_into()));
            }
        } else {
            let meta = use_head(cx);
            meta.stylesheets.els.borrow_mut().insert((id,href), None);
        }
    }
}
