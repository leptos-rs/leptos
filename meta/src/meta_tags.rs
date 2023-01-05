use cfg_if::cfg_if;
use leptos::{component, IntoView, Scope};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

use crate::{use_head, TextProp};

/// Manages all of the `<meta>` elements set by [Meta] components.
#[derive(Clone, Default, Debug)]
pub struct MetaTagsContext {
    next_id: Cell<MetaTagId>,
    #[allow(clippy::type_complexity)]
    els: Rc<RefCell<HashMap<MetaTagId, (Option<MetaTag>, Option<web_sys::HtmlMetaElement>)>>>,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
struct MetaTagId(usize);

impl MetaTagsContext {
    fn get_next_id(&self) -> MetaTagId {
        let current_id = self.next_id.get();
        let next_id = MetaTagId(current_id.0 + 1);
        self.next_id.set(next_id);
        next_id
    }
}

#[derive(Clone, Debug)]
enum MetaTag {
    Charset(TextProp),
    HttpEquiv {
        http_equiv: TextProp,
        content: Option<TextProp>,
    },
    Name {
        name: TextProp,
        content: TextProp,
    },
}

impl MetaTagsContext {
    /// Converts the set of `<meta>` elements into an HTML string that can be injected into the `<head>`.
    pub fn as_string(&self) -> String {
        self.els
            .borrow()
            .iter()
            .filter_map(|(id, (tag, _))| {
				tag.as_ref().map(|tag| {
					let id = id.0;

					match tag {
						MetaTag::Charset(charset) => format!(r#"<meta charset="{}" data-leptos-meta="{id}">"#, charset.get()),
						MetaTag::HttpEquiv { http_equiv, content } => {
							if let Some(content) = &content {
								format!(r#"<meta http-equiv="{}" content="{}" data-leptos-meta="{id}">"#, http_equiv.get(), content.get())
							} else {
								format!(r#"<meta http-equiv="{}" data-leptos-meta="{id}">"#, http_equiv.get())
							}
						},
						MetaTag::Name { name, content } => format!(r#"<meta name="{}" content="{}" data-leptos-meta="{id}">"#, name.get(), content.get()),
					}
				})
			})
            .collect()
    }
}

/// Injects an [HTMLMetaElement](https://developer.mozilla.org/en-US/docs/Web/API/HTMLMetaElement) into the document
/// head to set metadata
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
///       <Meta charset="utf-8"/>
///       <Meta name="description" content="A Leptos fan site."/>
///       <Meta http_equiv="refresh" content="3;url=https://github.com/gbj/leptos"/>
///     </main>
///   }
/// }
/// ```
#[component(transparent)]
pub fn Meta(
    cx: Scope,
    /// The [`charset`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meta#attr-charset) attribute.
    #[prop(optional, into)]
    charset: Option<TextProp>,
    /// The [`name`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meta#attr-name) attribute.
    #[prop(optional, into)]
    name: Option<TextProp>,
    /// The [`http-equiv`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meta#attr-http-equiv) attribute.
    #[prop(optional, into)]
    http_equiv: Option<TextProp>,
    /// The [`content`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meta#attr-content) attribute.
    #[prop(optional, into)]
    content: Option<TextProp>,
) -> impl IntoView {
    let tag = match (charset, name, http_equiv, content) {
		(Some(charset), _, _, _) => MetaTag::Charset(charset),
		(_, _, Some(http_equiv), content) => MetaTag::HttpEquiv { http_equiv, content },
		(_, Some(name), _, Some(content)) => MetaTag::Name { name, content },
		_ => panic!("<Meta/> tag expects either `charset`, `http_equiv`, or `name` and `content` to be set.")
	};

    cfg_if! {
        if #[cfg(any(feature = "csr", feature = "hydrate"))] {
            use leptos::{document, JsCast, UnwrapThrowExt, create_effect};

            let meta = use_head(cx);
            let meta_tags = meta.meta_tags;
            let id = meta_tags.get_next_id();

            let el = if let Ok(Some(el)) = document().query_selector(&format!("[data-leptos-meta='{}']", id.0)) {
                el
            } else {
                document().create_element("meta").unwrap_throw()
            };

            match tag {
                MetaTag::Charset(charset) => {
                    create_effect(cx, {
                        let el = el.clone();
                        move |_| {
                            _ = el.set_attribute("charset", &charset.get());
                        }
                    })
                },
                MetaTag::HttpEquiv { http_equiv, content } => {
                    create_effect(cx, {
                        let el = el.clone();
                        move |_| {
                            _ = el.set_attribute("http-equiv", &http_equiv.get());
                        }
                    });
                    if let Some(content) = content {
                        create_effect(cx, {
                            let el = el.clone();
                            move |_| {
                                _ = el.set_attribute("content", &content.get());
                            }
                        });
                    }
                },
                MetaTag::Name { name, content } => {
                    create_effect(cx, {
                        let el = el.clone();
                        move |_| {
                            _ = el.set_attribute("name", &name.get());
                        }
                    });
                    create_effect(cx, {
                        let el = el.clone();
                        move |_| {
                            _ = el.set_attribute("content", &content.get());
                        }
                    });
                },
            }

            // add to head
            let head = document()
                .query_selector("head")
                .unwrap_throw()
                .unwrap_throw();
            head.append_child(&el)
                .unwrap_throw();

            leptos::on_cleanup(cx, {
                let el = el.clone();
                move || {
                    head.remove_child(&el);
                }
            });

            // add to meta tags
            meta_tags.els.borrow_mut().insert(id, (None, Some(el.unchecked_into())));
        } else {
            let meta = use_head(cx);
            let meta_tags = meta.meta_tags;
            meta_tags.els.borrow_mut().insert(meta_tags.get_next_id(), (Some(tag), None));
        }
    }
}
