use cfg_if::cfg_if;
use leptos::*;
#[cfg(feature = "ssr")]
use std::collections::HashMap;
#[cfg(feature = "ssr")]
use std::{cell::RefCell, rc::Rc};

/// Contains the current metadata for the document's `<body>`.
#[derive(Clone, Default)]
pub struct BodyContext {
    #[cfg(feature = "ssr")]
    class: Rc<RefCell<Option<TextProp>>>,
    #[cfg(feature = "ssr")]
    id: Rc<RefCell<Option<TextProp>>>,
    #[cfg(feature = "ssr")]
    attributes: Rc<RefCell<HashMap<&'static str, Attribute>>>,
}

impl BodyContext {
    /// Converts the `<body>` metadata into an HTML string.
    #[cfg(any(feature = "ssr", doc))]
    pub fn as_string(&self) -> Option<String> {
        let class = self.class.borrow().as_ref().map(|val| {
            format!(
                "class=\"{}\"",
                leptos::leptos_dom::ssr::escape_attr(&val.get())
            )
        });

        let id = self.id.borrow().as_ref().map(|val| {
            format!(
                "id=\"{}\"",
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

        let mut val = [id, class, attributes]
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

impl core::fmt::Debug for BodyContext {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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
/// fn MyApp() -> impl IntoView {
///     provide_meta_context();
///     let (prefers_dark, set_prefers_dark) = create_signal(false);
///     let body_class = move || {
///         if prefers_dark.get() {
///             "dark".to_string()
///         } else {
///             "light".to_string()
///         }
///     };
///
///     view! {
///       <main>
///         <Body class=body_class attr:class="foo"/>
///       </main>
///     }
/// }
/// ```
#[component(transparent)]
pub fn Body(
    /// The `class` attribute on the `<body>`.
    #[prop(optional, into)]
    class: Option<TextProp>,
    /// The `id` attribute on the `<body>`.
    #[prop(optional, into)]
    id: Option<TextProp>,
    /// Arbitrary attributes to add to the `<body>`
    #[prop(attrs)]
    attributes: Vec<(&'static str, Attribute)>,
) -> impl IntoView {
    cfg_if! {
        if #[cfg(all(target_arch = "wasm32", any(feature = "csr", feature = "hydrate")))] {
            use wasm_bindgen::JsCast;

            let el = document().body().expect("there to be a <body> element");

            if let Some(class) = class {
                create_render_effect({
                    let el = el.clone();
                    move |_| {
                        let value = class.get();
                        _ = el.set_attribute("class", &value);
                    }
                });
            }


            if let Some(id) = id {
                create_render_effect({
                    let el = el.clone();
                    move |_| {
                        let value = id.get();
                        _ = el.set_attribute("id", &value);
                    }
                });
            }
            for (name, value) in attributes {
                leptos::leptos_dom::attribute_helper(el.unchecked_ref(), name.into(), value);
            }
        } else if #[cfg(feature = "ssr")] {
            let meta = crate::use_head();
            *meta.body.class.borrow_mut() = class;
            *meta.body.id.borrow_mut() = id;
            meta.body.attributes.borrow_mut().extend(attributes);
        } else {
            _ = class;
            _ = id;
            _ = attributes;

            #[cfg(debug_assertions)]
            crate::feature_warning();
        }
    }
}
