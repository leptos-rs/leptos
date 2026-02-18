use super::{
    component_builder::maybe_optimised_component_children,
    convert_to_snake_case, full_path_from_tag_name,
    utils::{attr_check_info, children_span},
};
use crate::view::{fragment_to_tokens, utils::filter_prefixed_attrs, TagType};
use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::{quote, quote_spanned};
use rstml::node::{CustomNode, KeyedAttribute, NodeAttribute, NodeElement};
use std::collections::HashMap;
use syn::spanned::Spanned;

pub(crate) fn slot_to_tokens(
    node: &mut NodeElement<impl CustomNode>,
    slot: &KeyedAttribute,
    parent_slots: Option<&mut HashMap<String, Vec<TokenStream>>>,
    global_class: Option<&TokenTree>,
    disable_inert_html: bool,
) {
    let name = slot.key.to_string();
    let name = name.trim();
    let name = convert_to_snake_case(if name.starts_with("slot:") {
        name.replacen("slot:", "", 1)
    } else {
        node.name().to_string()
    });

    let component_path = full_path_from_tag_name(node.name());

    let Some(parent_slots) = parent_slots else {
        proc_macro_error2::emit_error!(
            node.name().span(),
            "slots cannot be used inside HTML elements"
        );
        return;
    };

    let attrs = node
        .attributes()
        .iter()
        .filter_map(|node| {
            if let NodeAttribute::Attribute(node) = node {
                if is_slot(node) {
                    None
                } else {
                    Some(node)
                }
            } else {
                None
            }
        })
        .cloned()
        .collect::<Vec<_>>();

    // Collect pre-check info and builder setter info
    let mut pre_check_info: Vec<(
        Ident,       // check_fn ident with check_span
        Ident,       // checked_var ident with check_span
        TokenStream, // value expression
        Span,        // check_span
    )> = vec![];
    let mut builder_setters: Vec<TokenStream> = vec![];
    let props_with_checks: Vec<TokenStream> = attrs
        .iter()
        .filter(|attr| {
            !attr.key.to_string().starts_with("let:")
                && !attr.key.to_string().starts_with("clone:")
                && !attr.key.to_string().starts_with("attr:")
        })
        .map(|attr| {
            let attr_name = &attr.key;

            let value = attr
                .value()
                .map(|v| {
                    quote! { #v }
                })
                .unwrap_or_else(|| quote! { #attr_name });

            let (check_fn, check_span) = attr_check_info(attr);
            let check_fn_name_str = check_fn.to_string();
            let clean_prop = check_fn_name_str
                .strip_prefix("__check_")
                .unwrap_or(&check_fn_name_str);
            let checked_var = Ident::new(
                &format!("__checked_{clean_prop}"),
                check_span,
            );
            let check_fn_spanned =
                Ident::new(&check_fn_name_str, check_span);

            pre_check_info.push((
                check_fn_spanned,
                checked_var.clone(),
                value,
                check_span,
            ));

            quote! {
                let __props_builder = __props_builder.#attr_name(#[allow(unused_braces)] #checked_var);
            }
        })
        .collect();
    builder_setters = props_with_checks;

    let items_to_bind = filter_prefixed_attrs(attrs.iter(), "let:")
        .into_iter()
        .map(|ident| quote! { #ident })
        .collect::<Vec<_>>();

    let items_to_clone = filter_prefixed_attrs(attrs.iter(), "clone:");

    let dyn_attrs = attrs
        .iter()
        .filter(|attr| attr.key.to_string().starts_with("attr:"))
        .filter_map(|attr| {
            let name = &attr.key.to_string();
            let name = name.strip_prefix("attr:");
            let value = attr.value().map(|v| {
                quote! { #v }
            })?;
            Some(quote! { (#name, #value) })
        })
        .collect::<Vec<_>>();

    let dyn_attrs = if dyn_attrs.is_empty() {
        quote! {}
    } else {
        quote! { .dyn_attrs(vec![#(#dyn_attrs),*]) }
    };

    // Compute children span once, used for both the children arg
    // and the pre-check.
    let name_span = node.name().span();
    let children_span = children_span(&node.children, name_span);

    let mut slots = HashMap::new();
    // Extract children arg expression (without builder call wrapper)
    let children_arg: Option<TokenStream> = if node.children.is_empty() {
        None
    } else if let Some(children_arg) = maybe_optimised_component_children(
        &node.children,
        &items_to_bind,
        &items_to_clone,
    ) {
        Some(children_arg)
    } else {
        let children = fragment_to_tokens(
            &mut node.children,
            TagType::Unknown,
            Some(&mut slots),
            global_class,
            None,
            disable_inert_html,
        );

        if let Some(children) = children {
            let bindables =
                items_to_bind.iter().map(|ident| quote! { #ident, });

            let clonables = items_to_clone.iter().map(|ident| {
                quote_spanned! {ident.span()=>
                    let #ident = ::core::clone::Clone::clone(&#ident);
                }
            });

            if bindables.len() > 0 {
                Some(quote_spanned! {children_span=>
                    {
                        #(#clonables)*

                        move |#(#bindables)*| #children
                    }
                })
            } else {
                Some(quote_spanned! {children_span=>
                    {
                        #(#clonables)*

                        ::leptos::children::ToChildren::to_children(move || #children)
                    }
                })
            }
        } else {
            None
        }
    };

    // Generate children pre-check and builder call
    let (children_pre_check, children_builder_call) = if let Some(ref arg) =
        children_arg
    {
        let pre_check = quote_spanned! {children_span=>
            let __checked_children = #component_path ::__check_children(#arg);
        };
        let builder_call = quote_spanned! {name_span=>
            let __props_builder = __props_builder.children(__checked_children);
        };
        (pre_check, builder_call)
    } else {
        (quote! {}, quote! {})
    };

    let slots = slots.drain().map(|(slot, mut values)| {
        let span = values
            .last()
            .expect("List of slots must not be empty")
            .span();
        let slot = Ident::new(&slot, span);
        let value = if values.len() > 1 {
            quote! {
                ::std::vec![
                    #(#values)*
                ]
            }
        } else {
            values.remove(0)
        };

        quote! {
            let __props_builder = __props_builder.#slot(#value);
        }
    });

    // Generate pre-check calls using slot struct
    let pre_checks: Vec<TokenStream> = pre_check_info
        .iter()
        .map(|(check_fn, checked_var, value, span)| {
            quote_spanned! {*span=>
                let #checked_var = #component_path ::#check_fn(
                    #[allow(unused_braces)] { #value }
                );
            }
        })
        .collect();

    let build = quote_spanned! {node.name().span()=>
        .build()
    };

    let slot = quote_spanned! {node.span()=>
        {
            #(#pre_checks)*
            #children_pre_check
            let __props_builder = #component_path::builder();
            #(#builder_setters)*
            #(#slots)*
            #children_builder_call
            let __props_builder = #component_path ::__check_missing(__props_builder);
            let slot = __props_builder #build;
            let slot = slot.__finalize();
            let slot = slot #dyn_attrs;

            #[allow(unreachable_code, clippy::useless_conversion)]
            slot.into()
        },
    };

    // We need to move "allow" out of "quote_spanned" because it breaks hovering in rust-analyzer
    let slot = quote!(#[allow(unused_braces)] #slot);

    parent_slots
        .entry(name)
        .and_modify(|entry| entry.push(slot.clone()))
        .or_insert(vec![slot]);
}

pub(crate) fn is_slot(node: &KeyedAttribute) -> bool {
    let key = node.key.to_string();
    let key = key.trim();
    key == "slot" || key.starts_with("slot:")
}

pub(crate) fn get_slot(
    node: &NodeElement<impl CustomNode>,
) -> Option<&KeyedAttribute> {
    node.attributes().iter().find_map(|node| {
        if let NodeAttribute::Attribute(node) = node {
            if is_slot(node) {
                Some(node)
            } else {
                None
            }
        } else {
            None
        }
    })
}
