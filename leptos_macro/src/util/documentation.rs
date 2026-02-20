use itertools::Itertools;
use leptos_hot_reload::parsing::value_to_string;
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens, TokenStreamExt};
use syn::{spanned::Spanned, Attribute, LitStr, Meta};

#[derive(Clone)]
pub struct Docs(Vec<(String, Span)>);

impl ToTokens for Docs {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let s = self
            .0
            .iter()
            .map(|(doc, span)| quote_spanned!(*span=> #[doc = #doc]))
            .collect::<TokenStream>();

        tokens.append_all(s);
    }
}

impl Docs {
    pub fn new(attrs: &[Attribute]) -> Self {
        #[derive(Debug, Copy, Clone, PartialEq, Eq)]
        enum ViewCodeFenceState {
            Outside,
            Rust,
            Rsx,
        }
        let mut quotes = "```".to_string();
        let mut quote_ws = "".to_string();
        let mut view_code_fence_state = ViewCodeFenceState::Outside;
        // todo fix docs stuff
        const RSX_START: &str = "# ::leptos::view! {";
        const RSX_END: &str = "# };";

        // Separated out of chain to allow rustfmt to work
        let map = |(doc, span): (String, Span)| {
            doc.split('\n')
                .map(str::trim_end)
                .flat_map(|doc| {
                    let trimmed_doc = doc.trim_start();
                    let leading_ws = &doc[..doc.len() - trimmed_doc.len()];
                    let trimmed_doc = trimmed_doc.trim_end();
                    match view_code_fence_state {
                        ViewCodeFenceState::Outside
                            if trimmed_doc.starts_with("```")
                                && trimmed_doc
                                    .trim_start_matches('`')
                                    .starts_with("view") =>
                        {
                            view_code_fence_state = ViewCodeFenceState::Rust;
                            let view = trimmed_doc.find('v').unwrap();
                            trimmed_doc[..view].clone_into(&mut quotes);
                            leading_ws.clone_into(&mut quote_ws);
                            let rust_options = &trimmed_doc
                                [view + "view".len()..]
                                .trim_start();
                            vec![
                                format!("{leading_ws}{quotes}{rust_options}"),
                                format!("{leading_ws}"),
                            ]
                        }
                        ViewCodeFenceState::Rust if trimmed_doc == quotes => {
                            view_code_fence_state = ViewCodeFenceState::Outside;
                            vec![format!("{leading_ws}"), doc.to_owned()]
                        }
                        ViewCodeFenceState::Rust
                            if trimmed_doc.starts_with('<') =>
                        {
                            view_code_fence_state = ViewCodeFenceState::Rsx;
                            vec![
                                format!("{leading_ws}{RSX_START}"),
                                doc.to_owned(),
                            ]
                        }
                        ViewCodeFenceState::Rsx if trimmed_doc == quotes => {
                            view_code_fence_state = ViewCodeFenceState::Outside;
                            vec![
                                format!("{leading_ws}{RSX_END}"),
                                doc.to_owned(),
                            ]
                        }
                        _ => vec![doc.to_string()],
                    }
                })
                .map(|l| (l, span))
                .collect_vec()
        };

        let mut attrs = attrs
            .iter()
            .filter_map(|attr| {
                let Meta::NameValue(attr) = &attr.meta else {
                    return None;
                };
                if !attr.path.is_ident("doc") {
                    return None;
                }

                let Some(val) = value_to_string(&attr.value) else {
                    abort!(
                        attr,
                        "expected string literal in value of doc comment"
                    );
                };

                Some((val, attr.path.span()))
            })
            .flat_map(map)
            .collect_vec();

        if view_code_fence_state != ViewCodeFenceState::Outside {
            if view_code_fence_state == ViewCodeFenceState::Rust {
                attrs.push((quote_ws.clone(), Span::call_site()))
            } else {
                attrs.push((format!("{quote_ws}{RSX_END}"), Span::call_site()))
            }
            attrs.push((format!("{quote_ws}{quotes}"), Span::call_site()))
        }

        Self(attrs)
    }

    pub fn padded(&self) -> TokenStream {
        self.0
            .iter()
            .enumerate()
            .map(|(idx, (doc, span))| {
                let doc = if idx == 0 {
                    format!("    - {doc}")
                } else {
                    format!("      {doc}")
                };

                let doc = LitStr::new(&doc, *span);

                quote! { #[doc = #doc] }
            })
            .collect()
    }

    pub fn typed_builder(&self) -> String {
        let doc_str = self.0.iter().map(|s| s.0.as_str()).join("\n");

        if doc_str.chars().filter(|c| *c != '\n').count() != 0 {
            format!("\n\n{doc_str}")
        } else {
            String::new()
        }
    }
}
