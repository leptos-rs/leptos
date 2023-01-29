use crate::TextProp;
use cfg_if::cfg_if;
use leptos::*;
use std::{cell::RefCell, rc::Rc};

/// Contains the current metadata for the document's `<body>`.
#[derive(Clone, Default)]
pub struct BodyContext {
    class: Rc<RefCell<Option<TextProp>>>,
}

impl BodyContext {
    /// Converts the <body> metadata into an HTML string.
    pub fn as_string(&self) -> Option<String> {
        self.class
            .borrow()
            .as_ref()
            .map(|class| format!(" class=\"{}\"", class.get()))
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
///   provide_meta_context(cx);
///   let (prefers_dark, set_prefers_dark) = create_signal(cx, false);
///
///   view! { cx,
///     <main>
///       <Body class=move || if prefers_dark() { "dark" } else { "light" }/>
///     </main>
///   }
/// }
/// ```
#[component(transparent)]
pub fn Body(
    cx: Scope,
    /// The `class` attribute on the `<body>`.
    #[prop(optional, into)]
    class: Option<TextProp>,
) -> impl IntoView {
    cfg_if! {
        if #[cfg(any(feature = "csr", feature = "hydrate"))] {
            let el = document().body().expect("there to be a <body> element");

            if let Some(class) = class {
                create_render_effect(cx, move |_| {
                    let value = class.get();
                    _ = el.set_attribute("class", &value);
                });
            }
        } else {
            let meta = crate::use_head(cx);
            *meta.body.class.borrow_mut() = class;
        }
    }
}
