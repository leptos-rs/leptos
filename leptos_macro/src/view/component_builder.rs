use super::utils::{
    children_span, delinked_path_from_node_name, extract_children_arg,
    generate_checked_builder_block, is_nostrip_optional_and_update_key,
    prop_span_info, turbofish_generics, PropInfo,
};
use crate::view::{
    attribute_absolute, text_to_tokens,
    utils::{filter_prefixed_attrs, key_value_span},
};
use proc_macro2::{Ident, TokenStream, TokenTree};
use quote::{format_ident, quote, quote_spanned};
use rstml::node::{
    CustomNode, KeyedAttributeValue, Node, NodeAttribute, NodeBlock,
    NodeElement, NodeName,
};
use std::collections::{HashMap, HashSet};
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

    // Capture component name tokens and span before mutable borrows
    let name_span = node.name().span();
    let component_path: TokenStream = {
        let n = node.name();
        quote! { #n }
    };

    // A span-delinked copy of the component path for builder and check calls.
    // The last segment gets `Span::call_site()` so that rust-analyzer does
    // NOT map ctrl+click on the source `<Component />` to the module usage
    // (which would cause a "choose function vs type" disambiguation
    // prompt).  Only the function reference (`&Component`) keeps the original
    // span, giving the IDE a single, unambiguous navigation target.
    let delinked_path = delinked_path_from_node_name(node.name(), "");

    // an attribute that contains {..} can be used to split props from
    // attributes anything before it is a prop, unless it uses the special
    // attribute syntaxes (attr:, style:, on:, prop:, etc.)
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

    // Initially using uncloned mutable reference, as the node.key might be
    // mutated during prop extraction (for nostrip:)
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

    // Collect pre-check info and builder setter info.
    //
    // For each non-optional prop we pre-check the value via UFCS
    // trait call through the companion module. For bounded generic
    // props, E0277 fires with custom `on_unimplemented` and the
    // expression type is `{error}`, suppressing downstream errors.
    let mut prop_infos: Vec<PropInfo> = vec![];
    let mut optional_props = vec![];
    let mut seen_prop_names = HashSet::new();
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

        let name_str = name.to_string();
        if !seen_prop_names.insert(name_str.clone()) {
            let msg = format!(
                "duplicate prop `{}` — each prop can only be set once",
                name_str
            );
            return quote_spanned! {attr.key.span()=>
                compile_error!(#msg)
            };
        }

        let value = attr
            .value()
            .map(|v| {
                quote! { #v }
            })
            .unwrap_or_else(|| quote! { #name });

        let key_value_span = key_value_span(
            attr.key.span(),
            attr.value().map(|it| it.span()),
            name.span(),
        );

        if optional {
            optional_props.push(quote_spanned! {key_value_span=>
                props.#name = { #value }.map(::leptos::prelude::IntoReactiveValue::into_reactive_value);
            })
        } else {
            let span_info = prop_span_info(attr);

            let setter_name = quote! { #name };
            prop_infos.push(PropInfo {
                span_info,
                value,
                setter_name,
                setter_span: key_value_span,
            });
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
                .add_any_attr({
                    vec![#(::leptos::attr::any_attribute::IntoAnyAttribute::into_any_attr(#spreads),)*]
                })
            }
        } else {
            quote! {
                .add_any_attr((#(#spreads,)*))
            }
        }
    });

    // Compute children span once, used for both the children arg
    // and the pre-check.
    let children_span = children_span(&node.children, name_span);

    let mut slots = HashMap::new();
    let children_arg = extract_children_arg(
        &mut node.children,
        &mut slots,
        &items_to_bind,
        &items_to_clone,
        children_span,
        global_class,
        disable_inert_html,
    );

    let generics = turbofish_generics(&node.open_tag.generics);
    let helper_init = quote! { #delinked_path ::__helper #generics () };
    // Use `name_span` so that where-clause failures on
    // `require_props()` and E0599 on `check_missing()` point
    // to the component name, not the entire `view!` invocation.
    let presence_ident = Ident::new("__presence", name_span);

    let builder_block = generate_checked_builder_block(
        helper_init,
        &presence_ident,
        &prop_infos,
        &mut slots,
        children_arg.as_ref(),
        children_span,
    );

    let props_ident = Ident::new("props", name_span);
    let props_mut = if optional_props.is_empty() {
        quote! {}
    } else {
        quote! { mut }
    };

    #[allow(unused_mut)] // used in debug
    let mut component = quote_spanned! {name_span=>
        {
            #[allow(unreachable_code)]
            #[allow(clippy::let_and_return)]
            ::leptos::component::component_view(
                #[allow(clippy::needless_borrows_for_generic_args)]
                &#component_path,
                {
                    #builder_block

                    // Build the final props value. `mut` keyword set if optional props must be set.
                    let #props_mut #props_ident = __props_builder.build();

                    // Call setters for optional props.
                    #(#optional_props)*

                    // Return the props value.
                    #props_ident
                }
            )
            #spreads
        }
    };

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
/// This is to work with all the variants of the
/// leptos::children::ToChildren::to_children trait. Strings are optimised to be
/// passed without the wrapping closure, providing significant compile time and
/// binary size improvements.
///
/// Returns just the children arg expression (not the full builder
/// call), or `None` if the children cannot be optimised.
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
        // If more than one child after filtering out comments, don't think we
        // can optimise:
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
                            if is_supported(&mac.mac) {
                                quote! { #block }
                            } else {
                                return None;
                            }
                        }
                        Stmt::Item(Item::Macro(mac)) => {
                            if is_supported(&mac.mac) {
                                quote! { #block }
                            } else {
                                return None;
                            }
                        }
                        Stmt::Expr(Expr::Macro(mac), _) => {
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

    let clonables = items_to_clone_to_tokens(items_to_clone);
    Some(quote_spanned! {children.span()=>
        {
            #(#clonables)*

            ::leptos::children::ToChildren::to_children(
                ::leptos::children::ChildrenOptContainer(#children),
            )
        }
    })
}
