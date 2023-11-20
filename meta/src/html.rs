use cfg_if::cfg_if;
use leptos::*;
#[cfg(feature = "ssr")]
use std::collections::HashMap;
#[cfg(feature = "ssr")]
use std::{cell::RefCell, rc::Rc};

/// Contains the current metadata for the document's `<html>`.
#[derive(Clone, Default)]
pub struct HtmlContext {
    #[cfg(feature = "ssr")]
    lang: Rc<RefCell<Option<TextProp>>>,
    #[cfg(feature = "ssr")]
    dir: Rc<RefCell<Option<TextProp>>>,
    #[cfg(feature = "ssr")]
    class: Rc<RefCell<Option<TextProp>>>,
    #[cfg(feature = "ssr")]
    attributes: Rc<RefCell<HashMap<&'static str, Attribute>>>,
}

impl HtmlContext {
    /// Converts the `<html>` metadata into an HTML string.
    #[cfg(any(feature = "ssr", doc))]
    pub fn as_string(&self) -> Option<String> {
        let lang = self.lang.borrow().as_ref().map(|val| {
            format!(
                "lang=\"{}\"",
                leptos::leptos_dom::ssr::escape_attr(&val.get())
            )
        });
        let dir = self.dir.borrow().as_ref().map(|val| {
            format!(
                "dir=\"{}\"",
                leptos::leptos_dom::ssr::escape_attr(&val.get())
            )
        });
        let class = self.class.borrow().as_ref().map(|val| {
            format!(
                "class=\"{}\"",
                leptos::leptos_dom::ssr::escape_attr(&val.get())
            )
        });
        let attributes = self.attributes.borrow();
        let attributes = (!attributes.is_empty()).then(|| {
            attributes
                .iter()
                .filter_map(|(n, v)| {
                    v.as_nameless_value_string().map(|v| {
                        format!(
                            "{}=\"{}\"",
                            n,
                            leptos::leptos_dom::ssr::escape_attr(&v)
                        )
                    })
                })
                .collect::<Vec<_>>()
                .join(" ")
        });
        let mut val = [lang, dir, class, attributes]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ");
        if val.is_empty() {
            None
        } else {
            val.insert(0, ' ');
            Some(val)
        }
    }
}

impl core::fmt::Debug for HtmlContext {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("TitleContext").finish()
    }
}

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
#[component(transparent)]
pub fn Html(
    /// The `lang` attribute on the `<html>`.
    #[prop(optional, into)]
    lang: Option<TextProp>,
    /// The `dir` attribute on the `<html>`.
    #[prop(optional, into)]
    dir: Option<TextProp>,
    /// The `class` attribute on the `<html>`.
    #[prop(optional, into)]
    class: Option<TextProp>,
    /// Arbitrary attributes to add to the `<html>`
    #[prop(attrs)]
    attributes: Vec<(&'static str, Attribute)>,
) -> impl IntoView {
    cfg_if! {
        if #[cfg(all(target_arch = "wasm32", any(feature = "csr", feature = "hydrate")))] {
            use wasm_bindgen::JsCast;

            let el = document().document_element().expect("there to be a <html> element");

            if let Some(lang) = lang {
                let el = el.clone();
                create_render_effect(move |_| {
                    let value = lang.get();
                    _ = el.set_attribute("lang", &value);
                });
            }

            if let Some(dir) = dir {
                let el = el.clone();
                create_render_effect(move |_| {
                    let value = dir.get();
                    _ = el.set_attribute("dir", &value);
                });
            }

            if let Some(class) = class {
                let el = el.clone();
                create_render_effect(move |_| {
                    let value = class.get();
                    _ = el.set_attribute("class", &value);
                });
            }

            for (name, value) in attributes {
                leptos::leptos_dom::attribute_helper(el.unchecked_ref(), name.into(), value);
            }
        } else if #[cfg(feature = "ssr")] {
            let meta = crate::use_head();
            if lang.is_some() {
                *meta.html.lang.borrow_mut() = lang;
            }
            if dir.is_some() {
                *meta.html.dir.borrow_mut() = dir;
            }
            if class.is_some() {
                *meta.html.class.borrow_mut() = class;
            }
            meta.html.attributes.borrow_mut().extend(attributes);
        } else {
                        _ = lang;
            _ = dir;
            _ = class;
            _ = attributes;
            #[cfg(debug_assertions)]
            crate::feature_warning();
        }
    }
}
