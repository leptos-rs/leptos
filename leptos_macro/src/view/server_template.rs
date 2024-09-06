use super::{
    camel_case_tag_name,
    component_builder::component_to_tokens,
    fancy_class_name, fancy_style_name,
    ide_helper::IdeTagHelper,
    is_custom_element, is_math_ml_element, is_self_closing, is_svg_element,
    parse_event_name,
    slot_helper::{get_slot, slot_to_tokens},
};
use crate::attribute_value;
use leptos_hot_reload::parsing::{
    block_to_primitive_expression, is_component_node, value_to_string,
};
use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::{quote, quote_spanned};
use rstml::node::{
    KeyedAttribute, Node, NodeAttribute, NodeBlock, NodeElement,
};
use std::collections::HashMap;
use syn::spanned::Spanned;

pub(crate) enum SsrElementChunks {
    String {
        template: String,
        holes: Vec<TokenStream>,
    },
    View(TokenStream),
}

pub(crate) fn root_node_to_tokens_ssr(
    node: &Node,
    global_class: Option<&TokenTree>,
    view_marker: Option<String>,
) -> TokenStream {
    match node {
        Node::Fragment(fragment) => fragment_to_tokens_ssr(
            &fragment.children,
            global_class,
            view_marker,
        ),
        Node::Comment(_) | Node::Doctype(_) => quote! {},
        Node::Text(node) => {
            quote! {
                ::leptos::leptos_dom::html::text(#node)
            }
        }
        Node::RawText(r) => {
            let text = r.to_string_best();
            let text = syn::LitStr::new(&text, r.span());
            quote! {
                ::leptos::leptos_dom::html::text(#text)
            }
        }
        Node::Block(node) => {
            quote! {
                #node
            }
        }
        Node::Element(node) => {
            root_element_to_tokens_ssr(node, global_class, view_marker)
                .unwrap_or_default()
        }
    }
}

pub(crate) fn fragment_to_tokens_ssr(
    nodes: &[Node],
    global_class: Option<&TokenTree>,
    view_marker: Option<String>,
) -> TokenStream {
    let original_span = nodes
        .first()
        .zip(nodes.last())
        .and_then(|(first, last)| first.span().join(last.span()))
        .unwrap_or_else(Span::call_site);

    let view_marker = if let Some(marker) = view_marker {
        quote! { .with_view_marker(#marker) }
    } else {
        quote! {}
    };

    let nodes = nodes.iter().map(|node| {
        let node = root_node_to_tokens_ssr(node, global_class, None);

        quote! {
            ::leptos::IntoView::into_view(#[allow(unused_braces)] { #node })
        }
    });

    quote! {
        {
            ::leptos::Fragment::lazy(|| ::std::vec![
                #(#nodes),*
            ])
            #view_marker
        }
    }
}

pub(crate) fn root_element_to_tokens_ssr(
    node: &NodeElement,
    global_class: Option<&TokenTree>,
    view_marker: Option<String>,
) -> Option<TokenStream> {
    // TODO: simplify, this is checked twice, second time in `element_to_tokens_ssr` body
    if is_component_node(node) {
        if let Some(slot) = get_slot(node) {
            slot_to_tokens(node, slot, None, global_class);
            None
        } else {
            Some(component_to_tokens(node, global_class))
        }
    } else {
        let mut stmts_for_ide = IdeTagHelper::new();
        let mut exprs_for_compiler = Vec::<TokenStream>::new();

        let mut template = String::new();
        let mut holes = Vec::new();
        let mut chunks = Vec::new();
        element_to_tokens_ssr(
            node,
            None,
            &mut template,
            &mut holes,
            &mut chunks,
            &mut stmts_for_ide,
            &mut exprs_for_compiler,
            true,
            global_class,
        );

        // push final chunk
        if !template.is_empty() {
            chunks.push(SsrElementChunks::String { template, holes })
        }

        let chunks = chunks.into_iter().map(|chunk| match chunk {
            SsrElementChunks::String { template, holes } => {
                if holes.is_empty() {
                    let template = template.replace("\\{", "{").replace("\\}", "}");
                    quote! {
                        ::leptos::leptos_dom::html::StringOrView::String(#template.into())
                    }
                } else {
                let template = template.replace("\\{", "{{").replace("\\}", "}}");
                    quote! {
                        ::leptos::leptos_dom::html::StringOrView::String(
                            ::std::format!(
                                #template,
                                #(#holes),*
                            ).into()
                        )
                    }
                }
            }
            SsrElementChunks::View(view) => {
                quote! {
                    #[allow(unused_braces)]
                    {
                        let view = #view;
                        ::leptos::leptos_dom::html::StringOrView::View(::std::rc::Rc::new(move || view.clone()))
                    }
                }
            },
        });

        let tag_name = node.name().to_string();
        let is_custom_element = is_custom_element(&tag_name);

        // Use any other span instead of node.name.span(), to avoid misunderstanding in IDE.
        // We can use open_tag.span(), to provide similar (to name span) diagnostic
        // in case of expansion error, but it will also highlight "<" token.
        let typed_element_name = if is_custom_element {
            Ident::new("Custom", Span::call_site())
        } else {
            let camel_cased = camel_case_tag_name(
                tag_name
                    .trim_start_matches("svg::")
                    .trim_start_matches("math::")
                    .trim_end_matches('_'),
            );
            Ident::new(&camel_cased, Span::call_site())
        };
        let typed_element_name = if is_svg_element(&tag_name) {
            quote! { svg::#typed_element_name }
        } else if is_math_ml_element(&tag_name) {
            quote! { math::#typed_element_name }
        } else {
            quote! { html::#typed_element_name }
        };
        let full_name = if is_custom_element {
            quote! {
                ::leptos::leptos_dom::html::Custom::new(#tag_name)
            }
        } else {
            quote! {
                <::leptos::leptos_dom::#typed_element_name as ::std::default::Default>::default()
            }
        };
        let view_marker = if let Some(marker) = view_marker {
            quote! { .with_view_marker(#marker) }
        } else {
            quote! {}
        };
        let stmts_for_ide = stmts_for_ide.into_iter();
        Some(quote! {
            #[allow(unused_braces)]
            {
                #(#stmts_for_ide)*
                #(#exprs_for_compiler)*
                ::leptos::HtmlElement::from_chunks(#full_name, [#(#chunks),*])#view_marker
            }
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn element_to_tokens_ssr(
    node: &NodeElement,
    parent_slots: Option<&mut HashMap<String, Vec<TokenStream>>>,
    template: &mut String,
    holes: &mut Vec<TokenStream>,
    chunks: &mut Vec<SsrElementChunks>,
    stmts_for_ide: &mut IdeTagHelper,
    exprs_for_compiler: &mut Vec<TokenStream>,
    is_root: bool,
    global_class: Option<&TokenTree>,
) {
    if is_component_node(node) {
        if let Some(slot) = get_slot(node) {
            slot_to_tokens(node, slot, parent_slots, global_class);
            return;
        }

        let component = component_to_tokens(node, global_class);

        if !template.is_empty() {
            chunks.push(SsrElementChunks::String {
                template: std::mem::take(template),
                holes: std::mem::take(holes),
            })
        }

        chunks.push(SsrElementChunks::View(quote! {
            ::leptos::IntoView::into_view(#[allow(unused_braces)] {#component})
        }));
    } else {
        let tag_name = node.name().to_string();
        let tag_name = tag_name
            .trim_start_matches("svg::")
            .trim_start_matches("math::")
            .trim_end_matches('_');
        let is_script_or_style = tag_name == "script" || tag_name == "style";
        template.push('<');
        template.push_str(tag_name);

        #[cfg(debug_assertions)]
        stmts_for_ide.save_element_completion(node);

        let mut inner_html = None;

        for attr in node.attributes() {
            if let NodeAttribute::Attribute(attr) = attr {
                inner_html = attribute_to_tokens_ssr(
                    attr,
                    template,
                    holes,
                    exprs_for_compiler,
                    global_class,
                );
            }
        }
        for attr in node.attributes() {
            use syn::{Expr, ExprRange, RangeLimits, Stmt};

            if let NodeAttribute::Block(NodeBlock::ValidBlock(block)) = attr {
                if let Some(Stmt::Expr(
                    Expr::Range(ExprRange {
                        start: None,
                        limits: RangeLimits::HalfOpen(_),
                        end: Some(end),
                        ..
                    }),
                    _,
                )) = block.stmts.first()
                {
                    // should basically be the resolved attributes, joined on spaces, placed into
                    // the template
                    template.push_str(" {}");
                    let end_into_iter =
                        quote_spanned!(end.span()=> {#end}.into_iter());
                    holes.push(quote_spanned! {block.span()=>
                        #end_into_iter.filter_map(|(name, attr)| {
                           Some(::std::format!(
                                "{}=\"{}\"",
                                name,
                                ::leptos::leptos_dom::ssr::escape_attr(&attr.as_nameless_value_string()?)
                            ))
                        }).collect::<::std::vec::Vec<_>>().join(" ")
                    });
                };
            }
        }

        // insert hydration ID
        let hydration_id = if is_root {
            quote! { ::leptos::leptos_dom::HydrationCtx::peek() }
        } else {
            quote! { ::leptos::leptos_dom::HydrationCtx::id() }
        };
        template.push_str("{}");
        holes.push(quote! {
            #hydration_id.map(|id| ::std::format!(" data-hk=\"{id}\"")).unwrap_or_default()
        });

        set_class_attribute_ssr(node, template, holes, global_class);
        set_style_attribute_ssr(node, template, holes);

        if is_self_closing(node) {
            template.push_str("/>");
        } else {
            template.push('>');

            if let Some(inner_html) = inner_html {
                template.push_str("{}");
                let value = inner_html;

                holes.push(quote! {
                  ::leptos::IntoAttribute::into_attribute(#value).as_nameless_value_string().unwrap_or_default()
                })
            } else {
                for child in &node.children {
                    match child {
                        Node::Element(child) => {
                            element_to_tokens_ssr(
                                child,
                                None,
                                template,
                                holes,
                                chunks,
                                stmts_for_ide,
                                exprs_for_compiler,
                                false,
                                global_class,
                            );
                        }
                        Node::Text(text) => {
                            let value = text.value_string();
                            let value = if is_script_or_style {
                                value.into()
                            } else {
                                html_escape::encode_safe(&value)
                            };
                            template.push_str(
                                &value.replace('{', "\\{").replace('}', "\\}"),
                            );
                        }
                        Node::RawText(r) => {
                            let value = r.to_string_best();
                            let value = if is_script_or_style {
                                value.into()
                            } else {
                                html_escape::encode_safe(&value)
                            };
                            template.push_str(
                                &value.replace('{', "\\{").replace('}', "\\}"),
                            );
                        }
                        Node::Block(NodeBlock::ValidBlock(block)) => {
                            if let Some(value) =
                                block_to_primitive_expression(block)
                                    .and_then(value_to_string)
                            {
                                template.push_str(&value);
                            } else {
                                if !template.is_empty() {
                                    chunks.push(SsrElementChunks::String {
                                        template: std::mem::take(template),
                                        holes: std::mem::take(holes),
                                    })
                                }
                                chunks.push(SsrElementChunks::View(quote! {
                                    ::leptos::IntoView::into_view(#block)
                                }));
                            }
                        }
                        // Keep invalid blocks for faster IDE diff (on user type)
                        Node::Block(block @ NodeBlock::Invalid { .. }) => {
                            chunks.push(SsrElementChunks::View(quote! {
                                ::leptos::IntoView::into_view(#block)
                            }));
                        }
                        Node::Fragment(_) => abort!(
                            Span::call_site(),
                            "You can't nest a fragment inside an element."
                        ),
                        Node::Comment(_) | Node::Doctype(_) => {}
                    }
                }
            }

            template.push_str("</");
            template.push_str(tag_name);
            template.push('>');
        }
    }
}

// returns `inner_html`
fn attribute_to_tokens_ssr<'a>(
    attr: &'a KeyedAttribute,
    template: &mut String,
    holes: &mut Vec<TokenStream>,
    exprs_for_compiler: &mut Vec<TokenStream>,
    global_class: Option<&TokenTree>,
) -> Option<&'a syn::Expr> {
    let name = attr.key.to_string();
    if name == "ref" || name == "_ref" || name == "ref_" || name == "node_ref" {
        // ignore refs on SSR
    } else if let Some(name) = name.strip_prefix("on:") {
        let handler = attribute_value(attr);
        let (event_type, _, _) = parse_event_name(name);

        exprs_for_compiler.push(quote! {
            ::leptos::leptos_dom::helpers::ssr_event_listener(::leptos::ev::#event_type, #handler);
        })
    } else if name.strip_prefix("prop:").is_some()
        || name.strip_prefix("class:").is_some()
        || name.strip_prefix("style:").is_some()
    {
        // ignore props for SSR
        // ignore classes and sdtyles: we'll handle these separately
        if name.starts_with("prop:") {
            let value = attr.value();
            exprs_for_compiler.push(quote! {
                #[allow(unused_braces)]
                { _ = #value; }
            });
        }
    } else if let Some(directive_name) = name.strip_prefix("use:") {
        let handler = syn::Ident::new(directive_name, attr.key.span());
        let value = attr.value();
        let value = value.map(|value| {
            quote! {
                _ = #value;
            }
        });
        exprs_for_compiler.push(quote! {
            #[allow(unused_braces)]
            {
                _ = #handler;
                #value
            }
        });
    } else if name == "inner_html" {
        return attr.value();
    } else {
        let name = name.replacen("attr:", "", 1);

        // special case of global_class and class attribute
        if name == "class"
            && global_class.is_some()
            && attr.value().and_then(value_to_string).is_none()
        {
            let span = attr.key.span();
            proc_macro_error2::emit_error!(span, "Combining a global class (view! { class = ... }) \
            and a dynamic `class=` attribute on an element causes runtime inconsistencies. You can \
            toggle individual classes dynamically with the `class:name=value` syntax. \n\nSee this issue \
            for more information and an example: https://github.com/leptos-rs/leptos/issues/773")
        };

        if name != "class" && name != "style" {
            template.push(' ');

            if let Some(value) = attr.value() {
                if let Some(value) = value_to_string(value) {
                    template.push_str(&name);
                    template.push_str("=\"");
                    template.push_str(&html_escape::encode_quoted_attribute(
                        &value,
                    ));
                    template.push('"');
                } else {
                    template.push_str("{}");
                    holes.push(quote! {
                        &::leptos::IntoAttribute::into_attribute(#value)
                            .as_nameless_value_string()
                            .map(|a| ::std::format!(
                                "{}=\"{}\"",
                                #name,
                                ::leptos::leptos_dom::ssr::escape_attr(&a)
                            ))
                            .unwrap_or_default()
                    })
                }
            } else {
                template.push_str(&name);
            }
        }
    };
    None
}

fn set_class_attribute_ssr(
    node: &NodeElement,
    template: &mut String,
    holes: &mut Vec<TokenStream>,
    global_class: Option<&TokenTree>,
) {
    let (static_global_class, dyn_global_class) = match global_class {
        Some(TokenTree::Literal(lit)) => {
            let str = lit.to_string();
            // A lit here can be a string, byte_string, char, byte_char, int or float.
            // If it's a string we remove the quotes so folks can use them directly
            // without needing braces. E.g. view!{class="my-class", ... }
            let str = if str.starts_with('"') && str.ends_with('"') {
                str[1..str.len() - 1].to_string()
            } else {
                str
            };
            (str, None)
        }
        None => (String::new(), None),
        Some(val) => (String::new(), Some(val)),
    };
    let static_class_attr = node
        .attributes()
        .iter()
        .filter_map(|a| match a {
            NodeAttribute::Attribute(attr)
                if attr.key.to_string() == "class" =>
            {
                attr.value().and_then(value_to_string)
            }
            _ => None,
        })
        .chain(Some(static_global_class))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    let dyn_class_attr = node
        .attributes()
        .iter()
        .filter_map(|a| {
            if let NodeAttribute::Attribute(a) = a {
                if a.key.to_string() == "class" {
                    if a.value().and_then(value_to_string).is_some()
                        || fancy_class_name(&a.key.to_string(), a).is_some()
                    {
                        None
                    } else {
                        Some((a.key.span(), a.value()))
                    }
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let class_attrs = node
        .attributes()
        .iter()
        .filter_map(|node| {
            if let NodeAttribute::Attribute(node) = node {
                let name = node.key.to_string();
                if name == "class" {
                    return if let Some((_, name, value)) =
                        fancy_class_name(&name, node)
                    {
                        let span = node.key.span();
                        Some((span, name, value))
                    } else {
                        None
                    };
                }
                if name.starts_with("class:") || name.starts_with("class-") {
                    let name = if name.starts_with("class:") {
                        name.replacen("class:", "", 1)
                    } else if name.starts_with("class-") {
                        name.replacen("class-", "", 1)
                    } else {
                        name
                    };
                    let value = attribute_value(node);
                    let span = node.key.span();
                    Some((span, name, value))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    if !static_class_attr.is_empty()
        || !dyn_class_attr.is_empty()
        || !class_attrs.is_empty()
        || dyn_global_class.is_some()
    {
        template.push_str(" class=\"");

        template.push_str(&html_escape::encode_quoted_attribute(
            &static_class_attr,
        ));

        for (_span, value) in dyn_class_attr {
            if let Some(value) = value {
                template.push_str(" {}");
                holes.push(quote! {
                  &::leptos::IntoAttribute::into_attribute(#value).as_nameless_value_string()
                    .map(|a| ::leptos::leptos_dom::ssr::escape_attr(&a).to_string())
                    .unwrap_or_default()
                });
            }
        }

        for (_span, name, value) in &class_attrs {
            template.push_str(" {}");
            holes.push(quote! {
                ::leptos::IntoClass::into_class(#value).as_value_string(#name)
            });
        }

        if let Some(dyn_global_class) = dyn_global_class {
            template.push_str(" {}");
            holes.push(quote! { #dyn_global_class });
        }

        template.push('"');
    }
}

fn set_style_attribute_ssr(
    node: &NodeElement,
    template: &mut String,
    holes: &mut Vec<TokenStream>,
) {
    let static_style_attr = node
        .attributes()
        .iter()
        .find_map(|a| match a {
            NodeAttribute::Attribute(attr)
                if attr.key.to_string() == "style" =>
            {
                attr.value().and_then(value_to_string)
            }
            _ => None,
        })
        .map(|style| format!("{style};"));

    let dyn_style_attr = node
        .attributes()
        .iter()
        .filter_map(|a| {
            if let NodeAttribute::Attribute(a) = a {
                if a.key.to_string() == "style" {
                    if a.value().and_then(value_to_string).is_some()
                        || fancy_style_name(&a.key.to_string(), a).is_some()
                    {
                        None
                    } else {
                        Some((a.key.span(), a.value()))
                    }
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let style_attrs = node
        .attributes()
        .iter()
        .filter_map(|node| {
            if let NodeAttribute::Attribute(node) = node {
                let name = node.key.to_string();
                if name == "style" {
                    return if let Some((_, name, value)) =
                        fancy_style_name(&name, node)
                    {
                        let span = node.key.span();
                        Some((span, name, value))
                    } else {
                        None
                    };
                }
                if name.starts_with("style:") || name.starts_with("style-") {
                    let name = if name.starts_with("style:") {
                        name.replacen("style:", "", 1)
                    } else if name.starts_with("style-") {
                        name.replacen("style-", "", 1)
                    } else {
                        name
                    };
                    let value = attribute_value(node);
                    let span = node.key.span();
                    Some((span, name, value))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    if static_style_attr.is_some()
        || !dyn_style_attr.is_empty()
        || !style_attrs.is_empty()
    {
        template.push_str(" style=\"");

        template.push_str(&static_style_attr.unwrap_or_default());

        for (_span, value) in dyn_style_attr {
            if let Some(value) = value {
                template.push_str(" {};");
                holes.push(quote! {
                  &::leptos::IntoAttribute::into_attribute(#value).as_nameless_value_string()
                    .map(|a| ::leptos::leptos_dom::ssr::escape_attr(&a).to_string())
                    .unwrap_or_default()
                });
            }
        }

        for (_span, name, value) in &style_attrs {
            template.push_str(" {}");
            holes.push(quote! {
                ::leptos::IntoStyle::into_style(#value).as_value_string(#name).unwrap_or_default()
            });
        }

        template.push('"');
    }
}
