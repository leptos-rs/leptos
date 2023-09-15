use super::{
    component_builder::component_to_tokens,
    expr_to_ident, fancy_class_name, fancy_style_name,
    ide_helper::IdeTagHelper,
    is_ambiguous_element, is_custom_element, is_math_ml_element,
    is_self_closing, is_svg_element, parse_event_name,
    slot_helper::{get_slot, slot_to_tokens},
};
use crate::attribute_value;
use leptos_hot_reload::parsing::{is_component_node, value_to_string};
use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::{quote, quote_spanned};
use rstml::node::{KeyedAttribute, Node, NodeAttribute, NodeElement, NodeName};
use std::collections::HashMap;
use syn::spanned::Spanned;

#[derive(Clone, Copy)]
pub(crate) enum TagType {
    Unknown,
    Html,
    Svg,
    Math,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn fragment_to_tokens(
    _span: Span,
    nodes: &[Node],
    lazy: bool,
    parent_type: TagType,
    parent_slots: Option<&mut HashMap<String, Vec<TokenStream>>>,
    global_class: Option<&TokenTree>,
    view_marker: Option<String>,
) -> Option<TokenStream> {
    let mut slots = HashMap::new();
    let has_slots = parent_slots.is_some();

    let mut nodes = nodes
        .iter()
        .filter_map(|node| {
            let node = node_to_tokens(
                node,
                parent_type,
                has_slots.then_some(&mut slots),
                global_class,
                None,
            )?;

            Some(quote! {
                #node.into_view()
            })
        })
        .peekable();

    if nodes.peek().is_none() {
        _ = nodes.collect::<Vec<_>>();
        if let Some(parent_slots) = parent_slots {
            for (slot, mut values) in slots.drain() {
                parent_slots
                    .entry(slot)
                    .and_modify(|entry| entry.append(&mut values))
                    .or_insert(values);
            }
        }
        return None;
    }

    let view_marker = if let Some(marker) = view_marker {
        quote! { .with_view_marker(#marker) }
    } else {
        quote! {}
    };

    let tokens = if lazy {
        quote! {
            {
                ::leptos::Fragment::lazy(|| ::std::vec![
                    #(#nodes),*
                ])
                #view_marker
            }
        }
    } else {
        quote! {
            {
                ::leptos::Fragment::new(::std::vec![
                    #(#nodes),*
                ])
                #view_marker
            }
        }
    };

    if let Some(parent_slots) = parent_slots {
        for (slot, mut values) in slots.drain() {
            parent_slots
                .entry(slot)
                .and_modify(|entry| entry.append(&mut values))
                .or_insert(values);
        }
    }

    Some(tokens)
}

pub(crate) fn node_to_tokens(
    node: &Node,
    parent_type: TagType,
    parent_slots: Option<&mut HashMap<String, Vec<TokenStream>>>,
    global_class: Option<&TokenTree>,
    view_marker: Option<String>,
) -> Option<TokenStream> {
    match node {
        Node::Fragment(fragment) => fragment_to_tokens(
            Span::call_site(),
            &fragment.children,
            true,
            parent_type,
            None,
            global_class,
            view_marker,
        ),
        Node::Comment(_) | Node::Doctype(_) => Some(quote! {}),
        Node::Text(node) => Some(quote! {
            ::leptos::leptos_dom::html::text(#node)
        }),
        Node::Block(node) => Some(quote! { #node }),
        Node::RawText(r) => {
            let text = r.to_string_best();
            let text = syn::LitStr::new(&text, r.span());
            Some(quote! { #text })
        }
        Node::Element(node) => element_to_tokens(
            node,
            parent_type,
            parent_slots,
            global_class,
            view_marker,
        ),
    }
}

pub(crate) fn element_to_tokens(
    node: &NodeElement,
    mut parent_type: TagType,
    parent_slots: Option<&mut HashMap<String, Vec<TokenStream>>>,
    global_class: Option<&TokenTree>,
    view_marker: Option<String>,
) -> Option<TokenStream> {
    let name = node.name();
    if is_component_node(node) {
        if let Some(slot) = get_slot(node) {
            slot_to_tokens(node, slot, parent_slots, global_class);
            None
        } else {
            Some(component_to_tokens(node, global_class))
        }
    } else {
        let tag = name.to_string();
        // collect close_tag name to emit semantic information for IDE.
        let mut ide_helper_close_tag = IdeTagHelper::new();
        let close_tag = node.close_tag.as_ref().map(|c| &c.name);
        let name = if is_custom_element(&tag) {
            let name = node.name().to_string();
            // link custom ident to name span for IDE docs
            let custom = Ident::new("custom", name.span());
            quote! { ::leptos::leptos_dom::html::#custom(::leptos::leptos_dom::html::Custom::new(#name)) }
        } else if is_svg_element(&tag) {
            parent_type = TagType::Svg;
            quote! { ::leptos::leptos_dom::svg::#name() }
        } else if is_math_ml_element(&tag) {
            parent_type = TagType::Math;
            quote! { ::leptos::leptos_dom::math::#name() }
        } else if is_ambiguous_element(&tag) {
            match parent_type {
                TagType::Unknown => {
                    // We decided this warning was too aggressive, but I'll leave it here in case we want it later
                    /* proc_macro_error::emit_warning!(name.span(), "The view macro is assuming this is an HTML element, \
                    but it is ambiguous; if it is an SVG or MathML element, prefix with svg:: or math::"); */
                    quote! {
                        ::leptos::leptos_dom::html::#name()
                    }
                }
                TagType::Html => {
                    quote! { ::leptos::leptos_dom::html::#name() }
                }
                TagType::Svg => {
                    quote! { ::leptos::leptos_dom::svg::#name() }
                }
                TagType::Math => {
                    quote! { ::leptos::leptos_dom::math::#name() }
                }
            }
        } else {
            parent_type = TagType::Html;
            quote! { ::leptos::leptos_dom::html::#name() }
        };

        if let Some(close_tag) = close_tag {
            ide_helper_close_tag.save_tag_completion(close_tag)
        }

        let attrs = node.attributes().iter().filter_map(|node| {
            if let NodeAttribute::Attribute(node) = node {
                let name = node.key.to_string();
                let name = name.trim();
                if name.starts_with("class:")
                    || fancy_class_name(name, node).is_some()
                    || name.starts_with("style:")
                    || fancy_style_name(name, node).is_some()
                {
                    None
                } else {
                    Some(attribute_to_tokens(node, global_class))
                }
            } else {
                None
            }
        });
        let spread_attrs = node.attributes().iter().filter_map(|node| {
            use rstml::node::NodeBlock;
            use syn::{Expr, ExprRange, RangeLimits, Stmt};

            if let NodeAttribute::Block(NodeBlock::ValidBlock(block)) = node {
                match block.stmts.first()? {
                    Stmt::Expr(
                        Expr::Range(ExprRange {
                            start: None,
                            limits: RangeLimits::HalfOpen(_),
                            end: Some(end),
                            ..
                        }),
                        _,
                    ) => Some(quote! { .attrs(#[allow(unused_brace)] {#end}) }),
                    _ => None,
                }
            } else {
                None
            }
        });
        let class_attrs = node.attributes().iter().filter_map(|node| {
            if let NodeAttribute::Attribute(node) = node {
                let name = node.key.to_string();
                if let Some((fancy, _, _)) = fancy_class_name(&name, node) {
                    Some(fancy)
                } else if name.trim().starts_with("class:") {
                    Some(attribute_to_tokens(node, global_class))
                } else {
                    None
                }
            } else {
                None
            }
        });
        let style_attrs = node.attributes().iter().filter_map(|node| {
            if let NodeAttribute::Attribute(node) = node {
                let name = node.key.to_string();
                if let Some((fancy, _, _)) = fancy_style_name(&name, node) {
                    Some(fancy)
                } else if name.trim().starts_with("style:") {
                    Some(attribute_to_tokens(node, global_class))
                } else {
                    None
                }
            } else {
                None
            }
        });
        let global_class_expr = match global_class {
            None => quote! {},
            Some(class) => {
                quote! {
                    .classes(
                        #[allow(unused_braces)]
                        {#class}
                    )
                }
            }
        };

        if is_self_closing(node) && !node.children.is_empty() {
            proc_macro_error::abort!(
                node.name().span(),
                format!(
                    "<{tag}> is a self-closing tag and cannot have children."
                )
            );
        }

        let children = node.children.iter().map(|node| {
            let (child, is_static) = match node {
                Node::Fragment(fragment) => (
                    fragment_to_tokens(
                        Span::call_site(),
                        &fragment.children,
                        true,
                        parent_type,
                        None,
                        global_class,
                        None,
                    )
                    .unwrap_or({
                        let span = Span::call_site();
                        quote_spanned! {
                            span => ::leptos::leptos_dom::Unit
                        }
                    }),
                    false,
                ),
                Node::Text(node) => (quote! { #node }, true),
                Node::RawText(node) => {
                    let text = node.to_string_best();
                    let text = syn::LitStr::new(&text, node.span());
                    (quote! { #text }, true)
                }
                Node::Block(node) => (
                    quote! {
                       #node
                    },
                    false,
                ),
                Node::Element(node) => (
                    element_to_tokens(
                        node,
                        parent_type,
                        None,
                        global_class,
                        None,
                    )
                    .unwrap_or_default(),
                    false,
                ),
                Node::Comment(_) | Node::Doctype(_) => (quote! {}, false),
            };
            if is_static {
                quote! {
                    .child(#child)
                }
            } else {
                quote! {
                    .child((#child))
                }
            }
        });
        let view_marker = if let Some(marker) = view_marker {
            quote! { .with_view_marker(#marker) }
        } else {
            quote! {}
        };
        let ide_helper_close_tag = ide_helper_close_tag.into_iter();
        Some(quote! {
            {
            #(#ide_helper_close_tag)*
            #name
                #(#attrs)*
                #(#spread_attrs)*
                #(#class_attrs)*
                #(#style_attrs)*
                #global_class_expr
                #(#children)*
                #view_marker
            }
        })
    }
}

pub(crate) fn attribute_to_tokens(
    node: &KeyedAttribute,
    global_class: Option<&TokenTree>,
) -> TokenStream {
    let span = node.key.span();
    let name = node.key.to_string();
    if name == "ref" || name == "_ref" || name == "ref_" || name == "node_ref" {
        let value = expr_to_ident(attribute_value(node));
        let node_ref = quote_spanned! { span => node_ref };

        quote! {
            .#node_ref(#value)
        }
    } else if let Some(name) = name.strip_prefix("on:") {
        let handler = attribute_value(node);

        let (event_type, is_custom, is_force_undelegated) =
            parse_event_name(name);

        let event_name_ident = match &node.key {
            NodeName::Punctuated(parts) => {
                if parts.len() >= 2 {
                    Some(&parts[1])
                } else {
                    None
                }
            }
            _ => unreachable!(),
        };
        let undelegated_ident = match &node.key {
            NodeName::Punctuated(parts) => parts.last().and_then(|last| {
                if last.to_string() == "undelegated" {
                    Some(last)
                } else {
                    None
                }
            }),
            _ => unreachable!(),
        };
        let on = match &node.key {
            NodeName::Punctuated(parts) => &parts[0],
            _ => unreachable!(),
        };
        let on = {
            let span = on.span();
            quote_spanned! {
                span => .on
            }
        };
        let event_type = if is_custom {
            event_type
        } else if let Some(ev_name) = event_name_ident {
            let span = ev_name.span();
            quote_spanned! {
                span => #ev_name
            }
        } else {
            event_type
        };

        let event_type = if is_force_undelegated {
            let undelegated = if let Some(undelegated) = undelegated_ident {
                let span = undelegated.span();
                quote_spanned! {
                    span => #undelegated
                }
            } else {
                quote! { undelegated }
            };
            quote! { ::leptos::ev::#undelegated(::leptos::ev::#event_type) }
        } else {
            quote! { ::leptos::ev::#event_type }
        };

        quote! {
            #on(#event_type, #handler)
        }
    } else if let Some(name) = name.strip_prefix("prop:") {
        let value = attribute_value(node);
        let prop = match &node.key {
            NodeName::Punctuated(parts) => &parts[0],
            _ => unreachable!(),
        };
        let prop = {
            let span = prop.span();
            quote_spanned! {
                span => .prop
            }
        };
        quote! {
            #prop(#name, #[allow(unused_braces)] {#value})
        }
    } else if let Some(name) = name.strip_prefix("class:") {
        let value = attribute_value(node);
        let class = match &node.key {
            NodeName::Punctuated(parts) => &parts[0],
            _ => unreachable!(),
        };
        let class = {
            let span = class.span();
            quote_spanned! {
                span => .class
            }
        };
        quote! {
            #class(#name, #[allow(unused_braces)] {#value})
        }
    } else if let Some(name) = name.strip_prefix("style:") {
        let value = attribute_value(node);
        let style = match &node.key {
            NodeName::Punctuated(parts) => &parts[0],
            _ => unreachable!(),
        };
        let style = {
            let span = style.span();
            quote_spanned! {
                span => .style
            }
        };
        quote! {
            #style(#name, #[allow(unused_braces)] {#value})
        }
    } else {
        let name = name.replacen("attr:", "", 1);

        if let Some((fancy, _, _)) = fancy_class_name(&name, node) {
            return fancy;
        }

        // special case of global_class and class attribute
        if name == "class"
            && global_class.is_some()
            && node.value().and_then(value_to_string).is_none()
        {
            let span = node.key.span();
            proc_macro_error::emit_error!(span, "Combining a global class (view! { class = ... }) \
            and a dynamic `class=` attribute on an element causes runtime inconsistencies. You can \
            toggle individual classes dynamically with the `class:name=value` syntax. \n\nSee this issue \
            for more information and an example: https://github.com/leptos-rs/leptos/issues/773")
        };

        // all other attributes
        let value = match node.value() {
            Some(value) => {
                quote! { #value }
            }
            None => quote_spanned! { span => "" },
        };

        let attr = match &node.key {
            NodeName::Punctuated(parts) => Some(&parts[0]),
            _ => None,
        };
        let attr = if let Some(attr) = attr {
            let span = attr.span();
            quote_spanned! {
                span => .attr
            }
        } else {
            quote! {
                .attr
            }
        };
        quote! {
            #attr(#name, (#value))
        }
    }
}
