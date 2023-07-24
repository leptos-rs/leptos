use crate::{use_head, TextProp};
use cfg_if::cfg_if;
use leptos::*;
use std::{cell::RefCell, rc::Rc};
#[cfg(any(feature = "csr", feature = "hydrate"))]
use wasm_bindgen::{JsCast, UnwrapThrowExt};

/// Contains the current state of the document's `<title>`.
#[derive(Clone, Default)]
pub struct TitleContext {
    #[cfg(any(feature = "csr", feature = "hydrate"))]
    el: Rc<RefCell<Option<web_sys::HtmlTitleElement>>>,
    formatter: Rc<RefCell<Option<Formatter>>>,
    text: Rc<RefCell<Option<TextProp>>>,
}

impl TitleContext {
    /// Converts the title into a string that can be used as the text content of a `<title>` tag.
    pub fn as_string(&self) -> Option<String> {
        let title = self.text.borrow().as_ref().map(|f| f.get());
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

/// A function that is applied to the text value before setting `document.title`.
#[repr(transparent)]
pub struct Formatter(Box<dyn Fn(String) -> String>);

impl<F> From<F> for Formatter
where
    F: Fn(String) -> String + 'static,
{
    #[inline(always)]
    fn from(f: F) -> Formatter {
        Formatter(Box::new(f))
    }
}

/// A component to set the document’s title by creating an [HTMLTitleElement](https://developer.mozilla.org/en-US/docs/Web/API/HTMLTitleElement).
///
/// The `title` and `formatter` can be set independently of one another. For example, you can create a root-level
/// `<Title formatter=.../>` that will wrap each of the text values of `<Title/>` components created lower in the tree.
///
/// ```
/// use leptos::*;
/// use leptos_meta::*;
///
/// #[component]
/// fn MyApp(cx: Scope) -> impl IntoView {
///     provide_meta_context(cx);
///     let formatter = |text| format!("{text} — Leptos Online");
///
///     view! { cx,
///       <main>
///         <Title formatter/>
///         // ... routing logic here
///       </main>
///     }
/// }
///
/// #[component]
/// fn PageA(cx: Scope) -> impl IntoView {
///     view! { cx,
///       <main>
///         <Title text="Page A"/> // sets title to "Page A — Leptos Online"
///       </main>
///     }
/// }
///
/// #[component]
/// fn PageB(cx: Scope) -> impl IntoView {
///     view! { cx,
///       <main>
///         <Title text="Page B"/> // sets title to "Page B — Leptos Online"
///       </main>
///     }
/// }
/// ```
#[component(transparent)]
pub fn Title(
    cx: Scope,
    /// A function that will be applied to any text value before it’s set as the title.
    #[prop(optional, into)]
    formatter: Option<Formatter>,
    /// Sets the the current `document.title`.
    #[prop(optional, into)]
    text: Option<TextProp>,
) -> impl IntoView {
    let meta = use_head(cx);

    cfg_if! {
        if #[cfg(any(feature = "csr", feature = "hydrate"))] {
            if let Some(formatter) = formatter {
                *meta.title.formatter.borrow_mut() = Some(formatter);
            }
            if let Some(text) = text {
                *meta.title.text.borrow_mut() = Some(text);
            }

            let el = {
                let mut el_ref = meta.title.el.borrow_mut();
                let el = if let Some(el) = &*el_ref {
                    let prev_text = el.inner_text();
                    on_cleanup(cx, {
                        let el = el.clone();
                        move || {
                            _ = el.set_text(&prev_text);
                        }
                    });

                    el.clone()
                } else {
                    match document().query_selector("title") {
                        Ok(Some(title)) => title.unchecked_into(),
                        _ => {
                            let el_ref = meta.title.el.clone();
                            let el = document().create_element("title").unwrap_throw();
                            let head = document().head().unwrap_throw();
                            head.append_child(el.unchecked_ref())
                                .unwrap_throw();

                            on_cleanup(cx, {
                                let el = el.clone();
                                move || {
                                    _ = head.remove_child(&el);
                                    *el_ref.borrow_mut() = None;
                                }
                            });


                            el.unchecked_into()
                        }
                    }
                };
                *el_ref = Some(el.clone().unchecked_into());

                el
            };

            create_render_effect(cx, move |_| {
                let text = meta.title.as_string().unwrap_or_default();

                el.set_text_content(Some(&text));
            });
        } else {
            if let Some(formatter) = formatter {
                *meta.title.formatter.borrow_mut() = Some(formatter);
            }
            if let Some(text) = text {
                *meta.title.text.borrow_mut() = Some(text);
            }
        }
    }
}
