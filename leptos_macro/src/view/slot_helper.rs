use super::{
    client_builder::{fragment_to_tokens, TagType},
    convert_to_snake_case, ident_from_tag_name,
};
use proc_macro2::{Ident, TokenStream, TokenTree};
use quote::{format_ident, quote};
use rstml::node::{KeyedAttribute, NodeAttribute, NodeElement};
use std::collections::HashMap;
use syn::spanned::Spanned;

pub(crate) fn slot_to_tokens(
    node: &NodeElement,
    slot: &KeyedAttribute,
    parent_slots: Option<&mut HashMap<String, Vec<TokenStream>>>,
    global_class: Option<&TokenTree>,
) {
    let name = slot.key.to_string();
    let name = name.trim();
    let name = convert_to_snake_case(if name.starts_with("slot:") {
        name.replacen("slot:", "", 1)
    } else {
        node.name().to_string()
    });

    let component_name = ident_from_tag_name(node.name());
    let span = node.name().span();

    let Some(parent_slots) = parent_slots else {
        proc_macro_error::emit_error!(
            span,
            "slots cannot be used inside HTML elements"
        );
        return;
    };

    let attrs = node.attributes().iter().filter_map(|node| {
        if let NodeAttribute::Attribute(node) = node {
            if is_slot(node) {
                None
            } else {
                Some(node)
            }
        } else {
            None
        }
    });

    let props = attrs
        .clone()
        .filter(|attr| {
            !attr.key.to_string().starts_with("bind:")
                && !attr.key.to_string().starts_with("clone:")
        })
        .map(|attr| {
            let name = &attr.key;

            let value = attr
                .value()
                .map(|v| {
                    quote! { #v }
                })
                .unwrap_or_else(|| quote! { #name });

            quote! {
                .#name(#[allow(unused_braces)] {#value})
            }
        });

    let items_to_bind = attrs
        .clone()
        .filter_map(|attr| {
            attr.key
                .to_string()
                .strip_prefix("bind:")
                .map(|ident| format_ident!("{ident}", span = attr.key.span()))
        })
        .collect::<Vec<_>>();

    let items_to_clone = attrs
        .clone()
        .filter_map(|attr| {
            attr.key
                .to_string()
                .strip_prefix("clone:")
                .map(|ident| format_ident!("{ident}", span = attr.key.span()))
        })
        .collect::<Vec<_>>();

    let mut slots = HashMap::new();
    let children = if node.children.is_empty() {
        quote! {}
    } else {
        cfg_if::cfg_if! {
            if #[cfg(debug_assertions)] {
                let marker = format!("<{component_name}/>-children");
                let view_marker = quote! { .with_view_marker(#marker) };
            } else {
                let view_marker = quote! {};
            }
        }

        let children = fragment_to_tokens(
            span,
            &node.children,
            true,
            TagType::Unknown,
            Some(&mut slots),
            global_class,
            None,
        );

        if let Some(children) = children {
            let bindables =
                items_to_bind.iter().map(|ident| quote! { #ident, });

            let clonables = items_to_clone
                .iter()
                .map(|ident| quote! { let #ident = #ident.clone(); });

            if bindables.len() > 0 {
                quote! {
                    .children({
                        #(#clonables)*

                        move |#(#bindables)*| #children #view_marker
                    })
                }
            } else {
                quote! {
                    .children({
                        #(#clonables)*

                        Box::new(move || #children #view_marker)
                    })
                }
            }
        } else {
            quote! {}
        }
    };

    let slots = slots.drain().map(|(slot, values)| {
        let slot = Ident::new(&slot, span);
        if values.len() > 1 {
            quote! {
                .#slot(::std::vec![
                    #(#values)*
                ])
            }
        } else {
            let value = &values[0];
            quote! { .#slot(#value) }
        }
    });

    let slot = quote! {
        #component_name::builder()
            #(#props)*
            #(#slots)*
            #children
            .build()
            .into(),
    };

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

pub(crate) fn get_slot(node: &NodeElement) -> Option<&KeyedAttribute> {
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
