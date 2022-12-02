use cfg_if::cfg_if;
use leptos::Scope;
use std::{rc::Rc, cell::{RefCell, Cell}, collections::HashMap};
use typed_builder::TypedBuilder;

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
		content: Option<TextProp>
	},
	Name {
		name: TextProp,
		content: TextProp
	}
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

/// Properties for the [Meta] component.
#[derive(TypedBuilder)]
pub struct MetaProps {
    /// The [`charset`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meta#attr-charset) attribute.
    #[builder(default, setter(strip_option, into))]
    charset: Option<TextProp>,
	/// The [`name`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meta#attr-name) attribute.
	#[builder(default, setter(strip_option, into))]
	name: Option<TextProp>,
	/// The [`http-equiv`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meta#attr-http-equiv) attribute.
	#[builder(default, setter(strip_option, into))]
	http_equiv: Option<TextProp>,
	/// The [`content`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meta#attr-content) attribute.
	#[builder(default, setter(strip_option, into))]
	content: Option<TextProp>,
}

/// Injects an [HTMLMetaElement](https://developer.mozilla.org/en-US/docs/Web/API/HTMLMetaElement) into the document
/// head to set metadata
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
///       <Meta charset="utf-8"/>
///       <Meta name="description" content="A Leptos fan site."/>
///       <Meta http_equiv="refresh" content="3;url=https://github.com/gbj/leptos"/>
///     </main>
///   }
/// }
/// ```
#[allow(non_snake_case)]
pub fn Meta(cx: Scope, props: MetaProps) {
    let MetaProps { charset, name, http_equiv, content } = props;

	let tag = match (charset, name, http_equiv, content) {
		(Some(charset), _, _, _) => MetaTag::Charset(charset),
		(_, _, Some(http_equiv), content) => MetaTag::HttpEquiv { http_equiv, content },
		(_, Some(name), _, Some(content)) => MetaTag::Name { name, content },
		_ => panic!("<Meta/> tag expects either `charset`, `http_equiv`, or `name` and `content` to be set.")
	};

    cfg_if! {
        if #[cfg(any(feature = "csr", feature = "hydrate"))] {
            use leptos::{document, JsCast, UnwrapThrowExt, create_element, create_effect, set_attribute};

            let meta = use_head(cx);
			let meta_tags = meta.meta_tags;
			let id = meta_tags.get_next_id();

			let el = if let Ok(Some(el)) = document().query_selector(&format!("[data-leptos-meta={}]", id.0)) {
				el
			} else {
				create_element("meta")
			};

			match tag {
				MetaTag::Charset(charset) => {
					create_effect(cx, {
						let el = el.clone();
						move |_| {
							set_attribute(&el, "charset", &charset.get());
						}
					})
				},
				MetaTag::HttpEquiv { http_equiv, content } => {
					create_effect(cx, {
						let el = el.clone();
						move |_| {
							set_attribute(&el, "http-equiv", &http_equiv.get());
						}
					});
					if let Some(content) = content {
						create_effect(cx, {
							let el = el.clone();
							move |_| {
								set_attribute(&el, "content", &content.get());
							}
						});
					}
				},
				MetaTag::Name { name, content } => {
					create_effect(cx, {
						let el = el.clone();
						move |_| {
							set_attribute(&el, "name", &name.get());
						}
					});
					create_effect(cx, {
						let el = el.clone();
						move |_| {
							set_attribute(&el, "content", &content.get());
						}
					});
				},
			}

			// add to head
			document()
                    .query_selector("head")
                    .unwrap_throw()
                    .unwrap_throw()
                    .append_child(&el)
                    .unwrap_throw();

			// add to meta tags
			meta_tags.els.borrow_mut().insert(id, (None, Some(el.unchecked_into())));
        } else {
            let meta = use_head(cx);
			let meta_tags = meta.meta_tags;
            meta_tags.els.borrow_mut().insert(meta_tags.get_next_id(), (Some(tag), None));
        }
    }
}
