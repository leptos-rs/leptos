use crate::{additional_attributes::AdditionalAttributes, TextProp};
use cfg_if::cfg_if;
use leptos::*;
use std::{cell::RefCell, rc::Rc};

/// Contains the current metadata for the document's `<html>`.
#[derive(Clone, Default)]
pub struct HtmlContext {
    lang: Rc<RefCell<Option<TextProp>>>,
    dir: Rc<RefCell<Option<TextProp>>>,
    class: Rc<RefCell<Option<TextProp>>>,
    attributes: Rc<RefCell<Option<MaybeSignal<AdditionalAttributes>>>>,
}

impl HtmlContext {
    /// Converts the `<html>` metadata into an HTML string.
    pub fn as_string(&self) -> Option<String> {
        let lang = self
            .lang
            .borrow()
            .as_ref()
            .map(|val| format!("lang=\"{}\"", val.get()));
        let dir = self
            .dir
            .borrow()
            .as_ref()
            .map(|val| format!("dir=\"{}\"", val.get()));
        let class = self
            .class
            .borrow()
            .as_ref()
            .map(|val| format!("class=\"{}\"", val.get()));
        let attributes = self.attributes.borrow().as_ref().map(|val| {
            val.with(|val| {
                val.0
                    .iter()
                    .map(|(n, v)| format!("{}=\"{}\"", n, v.get()))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
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

impl std::fmt::Debug for HtmlContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
/// fn MyApp(cx: Scope) -> impl IntoView {
///     provide_meta_context(cx);
///
///     view! { cx,
///       <main>
///         <Html
///           lang="he"
///           dir="rtl"
///           // arbitrary additional attributes can be passed via `attributes`
///           attributes=AdditionalAttributes::from(vec![("data-theme", "dark")])
///         />
///       </main>
///     }
/// }
/// ```
#[component(transparent)]
pub fn Html(
    cx: Scope,
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
    #[prop(optional, into)]
    attributes: Option<MaybeSignal<AdditionalAttributes>>,
) -> impl IntoView {
    #[cfg(debug_assertions)]
    crate::feature_warning();

    cfg_if! {
        if #[cfg(any(feature = "csr", feature = "hydrate"))] {
            let el = document().document_element().expect("there to be a <html> element");

            if let Some(lang) = lang {
                let el = el.clone();
                create_render_effect(cx, move |_| {
                    let value = lang.get();
                    _ = el.set_attribute("lang", &value);
                });
            }

            if let Some(dir) = dir {
                let el = el.clone();
                create_render_effect(cx, move |_| {
                    let value = dir.get();
                    _ = el.set_attribute("dir", &value);
                });
            }

            if let Some(class) = class {
                let el = el.clone();
                create_render_effect(cx, move |_| {
                    let value = class.get();
                    _ = el.set_attribute("class", &value);
                });
            }

            if let Some(attributes) = attributes {
                let attributes = attributes.get();
                for (attr_name, attr_value) in attributes.0.into_iter() {
                    let el = el.clone();
                    create_render_effect(cx, move |_|{
                        let value = attr_value.get();
                            _ = el.set_attribute(&attr_name, &value);
                    });
                }
            }
        } else {
            let meta = crate::use_head(cx);
            *meta.html.lang.borrow_mut() = lang;
            *meta.html.dir.borrow_mut() = dir;
            *meta.html.class.borrow_mut() = class;
            *meta.html.attributes.borrow_mut() = attributes;
        }
    }
}
