use crate::use_head;
use cfg_if::cfg_if;
use leptos::*;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

/// Manages all of the stylesheets set by [Stylesheet] components.
#[derive(Clone, Default, Debug)]
pub struct StylesheetContext {
    #[allow(clippy::type_complexity)]
    // key is (id, href)
    els: Rc<RefCell<HashMap<StyleSheetData, Option<web_sys::HtmlLinkElement>>>>,
    next_id: Rc<Cell<StylesheetId>>,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
struct StylesheetId(usize);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct StyleSheetData {
    id: String,
    href: String,
}

impl StylesheetContext {
    fn get_next_id(&self) -> StylesheetId {
        let current_id = self.next_id.get();
        let next_id = StylesheetId(current_id.0 + 1);
        self.next_id.set(next_id);
        next_id
    }
}

impl StylesheetContext {
    /// Converts the set of stylesheets into an HTML string that can be injected into the `<head>`.
    pub fn as_string(&self) -> String {
        self.els
            .borrow()
            .iter()
            .map(|(StyleSheetData { id, href }, _)| {
                format!(r#"<link rel="stylesheet" id="{id}" href="{href}">"#)
            })
            .collect()
    }
}

/// Injects an [HTMLLinkElement](https://developer.mozilla.org/en-US/docs/Web/API/HTMLLinkElement) into the document
/// head that loads a stylesheet from the URL given by the `href` property.
///
/// ```
/// use leptos::*;
/// use leptos_meta::*;
///
/// #[component]
/// fn MyApp(cx: Scope) -> impl IntoView {
///   provide_meta_context(cx);
///
///   view! { cx,
///     <main>
///       <Stylesheet href="/style.css"/>
///     </main>
///   }
/// }
/// ```
#[component(transparent)]
pub fn Stylesheet(
    cx: Scope,
    /// The URL at which the stylesheet is located.
    #[prop(into)]
    href: String,
    /// An ID for the stylesheet.
    #[prop(optional, into)]
    id: Option<String>,
) -> impl IntoView {
    let meta = use_head(cx);
    let stylesheets = &meta.stylesheets;
    let next_id = stylesheets.get_next_id();
    let id = id.unwrap_or_else(|| format!("leptos-style-{}", next_id.0));
    let key = StyleSheetData { id, href };

    cfg_if! {
        if #[cfg(any(feature = "csr", feature = "hydrate"))] {
            use leptos::document;

            let element_to_hydrate = document().get_element_by_id(&key.id);

            let el = element_to_hydrate.unwrap_or_else(|| {
                let el = document().create_element("link").unwrap_throw();
                el.set_attribute("rel", "stylesheet").unwrap_throw();
                el.set_attribute("id", &key.id).unwrap_throw();
                el.set_attribute("href", &key.href).unwrap_throw();
                let head = document().head().unwrap_throw();
                head
                    .append_child(el.unchecked_ref())
                    .unwrap_throw();

                el
            });

            on_cleanup(cx, {
                let el = el.clone();
                let els = meta.stylesheets.els.clone();
                let key = key.clone();
                move || {
                    let head = document().head().unwrap_throw();
                    _ = head.remove_child(&el);
                    els.borrow_mut().remove(&key);
                }
            });

            meta.stylesheets
                .els
                .borrow_mut()
                .insert(key, Some(el.unchecked_into()));

        } else {
            let meta = use_head(cx);
            meta.stylesheets.els.borrow_mut().insert(key, None);
        }
    }
}
