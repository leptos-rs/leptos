use cfg_if::cfg_if;
use leptos::*;
use std::{cell::RefCell, rc::Rc};

/// Contains the current metadata for the document's `<body>`.
#[derive(Clone, Default)]
pub struct BodyContext {
    class: Rc<RefCell<Option<TextProp>>>,
    attributes: Rc<RefCell<Option<MaybeSignal<AdditionalAttributes>>>>,
}

impl BodyContext {
    /// Converts the `<body>` metadata into an HTML string.
    pub fn as_string(&self) -> Option<String> {
        let class = self
            .class
            .borrow()
            .as_ref()
            .map(|val| format!("class=\"{}\"", val.get()));
        let attributes = self.attributes.borrow().as_ref().map(|val| {
            val.with(|val| {
                val.into_iter()
                    .map(|(n, v)| format!("{}=\"{}\"", n, v.get()))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
        });
        let mut val = [class, attributes]
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

impl std::fmt::Debug for BodyContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("TitleContext").finish()
    }
}

/// A component to set metadata on the document’s `<body>` element from
/// within the application.
///
/// ```
/// use leptos::*;
/// use leptos_meta::*;
///
/// #[component]
/// fn MyApp(cx: Scope) -> impl IntoView {
///     provide_meta_context(cx);
///     let (prefers_dark, set_prefers_dark) = create_signal(cx, false);
///     let body_class = move || {
///         if prefers_dark() {
///             "dark".to_string()
///         } else {
///             "light".to_string()
///         }
///     };
///
///     view! { cx,
///       <main>
///         <Body class=body_class/>
///       </main>
///     }
/// }
/// ```
#[component(transparent)]
pub fn Body(
    cx: Scope,
    /// The `class` attribute on the `<body>`.
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
            let el = document().body().expect("there to be a <body> element");

            if let Some(class) = class {
                create_render_effect(cx, {
                    let el = el.clone();
                    move |_| {
                        let value = class.get();
                        _ = el.set_attribute("class", &value);
                    }
                });
            }

            if let Some(attributes) = attributes {
                let attributes = attributes.get();
                for (attr_name, attr_value) in attributes.into_iter() {
                    let el = el.clone();
                    let attr_name = attr_name.to_owned();
                    let attr_value = attr_value.to_owned();
                    create_render_effect(cx, move |_|{
                        let value = attr_value.get();
                            _ = el.set_attribute(&attr_name, &value);
                    });
                }
            }
        } else {
            let meta = crate::use_head(cx);
            *meta.body.class.borrow_mut() = class;
            *meta.body.attributes.borrow_mut() = attributes;
        }
    }
}
