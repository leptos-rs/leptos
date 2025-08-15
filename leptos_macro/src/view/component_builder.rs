use super::{
    fragment_to_tokens, utils::is_nostrip_optional_and_update_key, TagType,
};
use crate::view::{
    attribute_absolute, text_to_tokens, utils::filter_prefixed_attrs,
};
use proc_macro2::{Ident, TokenStream, TokenTree};
use quote::{format_ident, quote, quote_spanned};
use rstml::node::{
    CustomNode, KeyedAttributeValue, Node, NodeAttribute, NodeBlock,
    NodeElement, NodeName,
};
use std::collections::HashMap;
use syn::{
    spanned::Spanned, Expr, ExprPath, ExprRange, Item, RangeLimits, Stmt,
};

pub(crate) fn component_to_tokens(
    node: &mut NodeElement<impl CustomNode>,
    global_class: Option<&TokenTree>,
    disable_inert_html: bool,
) -> TokenStream {
    #[allow(unused)] // TODO this is used by hot-reloading
    #[cfg(debug_assertions)]
    let component_name = super::ident_from_tag_name(node.name());

    // an attribute that contains {..} can be used to split props from attributes
    // anything before it is a prop, unless it uses the special attribute syntaxes
    // (attr:, style:, on:, prop:, etc.)
    // anything after it is a plain HTML attribute to be spread onto the prop
    let spread_marker = node
        .attributes()
        .iter()
        .position(|node| match node {
            NodeAttribute::Block(NodeBlock::ValidBlock(block)) => {
                matches!(
                    block.stmts.first(),
                    Some(Stmt::Expr(
                        Expr::Range(ExprRange {
                            start: None,
                            limits: RangeLimits::HalfOpen(_),
                            end: None,
                            ..
                        }),
                        _,
                    ))
                )
            }
            _ => false,
        })
        .unwrap_or_else(|| node.attributes().len());

    // Initially using uncloned mutable reference, as the node.key might be mutated during prop extraction (for nostrip:)
    let mut attrs = node
        .attributes_mut()
        .iter_mut()
        .filter_map(|node| {
            if let NodeAttribute::Attribute(node) = node {
                Some(node)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let mut required_props = vec![];
    let mut optional_props = vec![];
    for (_, attr) in attrs.iter_mut().enumerate().filter(|(idx, attr)| {
        idx < &spread_marker && {
            let attr_key = attr.key.to_string();
            !is_attr_let(&attr.key)
                && !attr_key.starts_with("clone:")
                && !attr_key.starts_with("class:")
                && !attr_key.starts_with("style:")
                && !attr_key.starts_with("attr:")
                && !attr_key.starts_with("prop:")
                && !attr_key.starts_with("on:")
                && !attr_key.starts_with("use:")
        }
    }) {
        let optional = is_nostrip_optional_and_update_key(&mut attr.key);
        let name = &attr.key;

        let value = attr
            .value()
            .map(|v| {
                quote! { #v }
            })
            .unwrap_or_else(|| quote! { #name });

        if optional {
            optional_props.push(quote! {
                props.#name = { #value }.map(Into::into);
            })
        } else {
            required_props.push(quote! {
                .#name(#[allow(unused_braces)] { #value })
            })
        }
    }

    // Drop the mutable reference to the node, go to an owned clone:
    let attrs = attrs.into_iter().map(|a| a.clone()).collect::<Vec<_>>();

    let items_to_bind = attrs
        .iter()
        .filter_map(|attr| {
            if !is_attr_let(&attr.key) {
                return None;
            }

            let KeyedAttributeValue::Binding(binding) = &attr.possible_value
            else {
                if let Some(ident) = attr.key.to_string().strip_prefix("let:") {
                    let span = match &attr.key {
                        NodeName::Punctuated(path) => path[1].span(),
                        _ => unreachable!(),
                    };
                    let ident1 = format_ident!("{ident}", span = span);
                    return Some(quote_spanned! { span => #ident1 });
                } else {
                    return None;
                }
            };

            let inputs = &binding.inputs;
            Some(quote! { #inputs })
        })
        .collect::<Vec<_>>();

    let items_to_clone = filter_prefixed_attrs(attrs.iter(), "clone:");

    // include all attribute that are either
    // 1) blocks ({..attrs} or {attrs}),
    // 2) start with attr: and can be used as actual attributes, or
    // 3) the custom attribute types (on:, class:, style:, prop:, use:)
    let spreads = node
        .attributes()
        .iter()
        .enumerate()
        .filter_map(|(idx, attr)| {
            if idx == spread_marker {
                return None;
            }

            if let NodeAttribute::Block(block) = attr {
                let dotted = if let NodeBlock::ValidBlock(block) = block {
                    match block.stmts.first() {
                        Some(Stmt::Expr(
                            Expr::Range(ExprRange {
                                start: None,
                                limits: RangeLimits::HalfOpen(_),
                                end: Some(end),
                                ..
                            }),
                            _,
                        )) => Some(quote! { #end }),
                        _ => None,
                    }
                } else {
                    None
                };
                Some(dotted.unwrap_or_else(|| {
                    quote! {
                        #node
                    }
                }))
            } else if let NodeAttribute::Attribute(node) = attr {
                attribute_absolute(node, idx >= spread_marker)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let spreads = (!(spreads.is_empty())).then(|| {
        if cfg!(feature = "__internal_erase_components") {
            quote! {
                .add_any_attr(vec![#(#spreads.into_any_attr(),)*])
            }
        } else {
            quote! {
                .add_any_attr((#(#spreads,)*))
            }
        }
    });

    /*let directives = attrs
        .clone()
        .filter_map(|attr| {
            attr.key
                .to_string()
                .strip_prefix("use:")
                .map(|ident| directive_call_from_attribute_node(attr, ident))
        })
        .collect::<Vec<_>>();

    let events_and_directives =
        events.into_iter().chain(directives).collect::<Vec<_>>(); */

    let mut slots = HashMap::new();
    let children = if node.children.is_empty() {
        quote! {}
    } else if let Some(children) = maybe_optimised_component_children(
        &node.children,
        &items_to_bind,
        &items_to_clone,
    ) {
        children
    } else {
        let children = fragment_to_tokens(
            &mut node.children,
            TagType::Unknown,
            Some(&mut slots),
            global_class,
            None,
            disable_inert_html,
        );

        // TODO view marker for hot-reloading
        /*
        cfg_if::cfg_if! {
            if #[cfg(debug_assertions)] {
                let marker = format!("<{component_name}/>-children");
                // For some reason spanning for `.children` breaks, unless `#view_marker`
                // is also covered by `children.span()`.
                let view_marker = quote_spanned!(children.span()=> .with_view_marker(#marker));
            } else {
                let view_marker = quote! {};
            }
        }
        */

        if let Some(children) = children {
            let bindables =
                items_to_bind.iter().map(|ident| quote! { #ident, });

            let clonables = items_to_clone_to_tokens(&items_to_clone);

            if bindables.len() > 0 {
                quote_spanned! {children.span()=>
                    .children({
                        #(#clonables)*

                        move |#(#bindables)*| #children
                    })
                }
            } else {
                quote_spanned! {children.span()=>
                    .children({
                        #(#clonables)*

                        ::leptos::children::ToChildren::to_children(move || #children)
                    })
                }
            }
        } else {
            quote! {}
        }
    };

    let slots = slots.drain().map(|(slot, mut values)| {
        let span = values
            .last()
            .expect("List of slots must not be empty")
            .span();
        let slot = Ident::new(&slot, span);
        let value = if values.len() > 1 {
            quote_spanned! {span=>
                ::std::vec![
                    #(#values)*
                ]
            }
        } else {
            values.remove(0)
        };

        quote! { .#slot(#value) }
    });

    let generics = &node.open_tag.generics;
    let generics = if generics.lt_token.is_some() {
        quote! { ::#generics }
    } else {
        quote! {}
    };

    let name = node.name();
    #[allow(unused_mut)] // used in debug
    let mut component = quote! {
        {
            #[allow(unreachable_code)]
            #[allow(unused_mut)]
            #[allow(clippy::let_and_return)]
            ::leptos::component::component_view(
                #[allow(clippy::needless_borrows_for_generic_args)]
                &#name,
                {
                    let mut props = ::leptos::component::component_props_builder(&#name #generics)
                        #(#required_props)*
                        #(#slots)*
                        #children
                        .build();
                    #(#optional_props)*
                    props
                }
            )
            #spreads
        }
    };

    // (Temporarily?) removed
    // See note on the function itself below.
    /* #[cfg(debug_assertions)]
    IdeTagHelper::add_component_completion(&mut component, node); */

    component
}

fn is_attr_let(key: &NodeName) -> bool {
    if key.to_string().starts_with("let:") {
        true
    } else if let NodeName::Path(ExprPath { path, .. }) = key {
        path.segments.len() == 1 && path.segments[0].ident == "let"
    } else {
        false
    }
}

pub fn items_to_clone_to_tokens(
    items_to_clone: &[Ident],
) -> impl Iterator<Item = TokenStream> + '_ {
    items_to_clone.iter().map(|ident| {
        let ident_ref = quote_spanned!(ident.span()=> &#ident);
        quote! { let #ident = ::core::clone::Clone::clone(#ident_ref); }
    })
}

/// By default all children are placed in an outer closure || #children.
/// This is to work with all the variants of the leptos::children::ToChildren::to_children trait.
/// Strings are optimised to be passed without the wrapping closure, providing significant compile time and binary size improvements.
pub fn maybe_optimised_component_children(
    children: &[Node<impl CustomNode>],
    items_to_bind: &[TokenStream],
    items_to_clone: &[Ident],
) -> Option<TokenStream> {
    // If there are bindables will have to be in a closure:
    if !items_to_bind.is_empty() {
        return None;
    }

    // Filter out comments:
    let mut children_iter = children
        .iter()
        .filter(|child| !matches!(child, Node::Comment(_)));

    let children = if let Some(child) = children_iter.next() {
        // If more than one child after filtering out comments, don't think we can optimise:
        if children_iter.next().is_some() {
            return None;
        }
        match child {
            Node::Text(text) => text_to_tokens(&text.value),
            Node::RawText(raw) => {
                let text = raw.to_string_best();
                let text = syn::LitStr::new(&text, raw.span());
                text_to_tokens(&text)
            }
            // Specifically allow std macros that produce strings:
            Node::Block(NodeBlock::ValidBlock(block)) => {
                fn is_supported(mac: &syn::Macro) -> bool {
                    for string_macro in ["format", "include_str"] {
                        if mac.path.is_ident(string_macro) {
                            return true;
                        }
                    }
                    false
                }
                if block.stmts.len() > 1 {
                    return None;
                } else if let Some(stmt) = block.stmts.first() {
                    match stmt {
                        Stmt::Macro(mac) => {
                            // eprintln!("Macro: {:?}", mac.mac.path);
                            if is_supported(&mac.mac) {
                                quote! { #block }
                            } else {
                                return None;
                            }
                        }
                        Stmt::Item(Item::Macro(mac)) => {
                            // eprintln!("Item Macro: {:?}", mac.mac.path);
                            if is_supported(&mac.mac) {
                                quote! { #block }
                            } else {
                                return None;
                            }
                        }
                        Stmt::Expr(Expr::Macro(mac), _) => {
                            // eprintln!("Expr Macro: {:?}", mac.mac.path);
                            if is_supported(&mac.mac) {
                                quote! { #block }
                            } else {
                                return None;
                            }
                        }
                        _ => return None,
                    }
                } else {
                    return Some(quote! {});
                }
            }
            _ => return None,
        }
    } else {
        return None;
    };

    // // Debug check to see how many use this optimisation:
    // static COUNT: std::sync::atomic::AtomicUsize =
    //     std::sync::atomic::AtomicUsize::new(0);
    // COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    // eprintln!(
    //     "Optimised children: {}",
    //     COUNT.load(std::sync::atomic::Ordering::Relaxed)
    // );

    let clonables = items_to_clone_to_tokens(items_to_clone);
    Some(quote_spanned! {children.span()=>
        .children({
            #(#clonables)*

            ::leptos::children::ToChildren::to_children(::leptos::children::ChildrenOptContainer(#children))
        })
    })
}
