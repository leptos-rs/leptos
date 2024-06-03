use super::{convert_to_snake_case, ident_from_tag_name};
use crate::view::{fragment_to_tokens, TagType};
use proc_macro2::{Ident, TokenStream, TokenTree};
use quote::{format_ident, quote, quote_spanned};
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

    let Some(parent_slots) = parent_slots else {
        proc_macro_error::emit_error!(
            node.name().span(),
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
            !attr.key.to_string().starts_with("let:")
                && !attr.key.to_string().starts_with("clone:")
                && !attr.key.to_string().starts_with("attr:")
        })
        .map(|attr| {
            let name = &attr.key;

            let value = attr
                .value()
                .map(|v| {
                    quote! { #v }
                })
                .unwrap_or_else(|| quote! { #name });

            let value = quote_spanned! {value.span()=>
                #[allow(unused_braces)] {#value}
            };

            quote_spanned! {attr.span()=>
                .#name(#value)
            }
        });

    let items_to_bind = attrs
        .clone()
        .filter_map(|attr| {
            attr.key
                .to_string()
                .strip_prefix("let:")
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

    let dyn_attrs = attrs
        .filter(|attr| attr.key.to_string().starts_with("attr:"))
        .filter_map(|attr| {
            let name = &attr.key.to_string();
            let name = name.strip_prefix("attr:");
            let value = attr.value().map(|v| {
                quote! { #v }
            })?;
            Some(quote! { (#name, ::leptos::IntoAttribute::into_attribute(#value)) })
        })
        .collect::<Vec<_>>();

    let dyn_attrs = if dyn_attrs.is_empty() {
        quote! {}
    } else {
        quote! { .dyn_attrs(vec![#(#dyn_attrs),*]) }
    };

    let mut slots = HashMap::new();
    let children = if node.children.is_empty() {
        quote! {}
    } else {
        let children = fragment_to_tokens(
            &node.children,
            TagType::Unknown,
            Some(&mut slots),
            global_class,
            None,
        );

        // TODO view markers for hot-reloading
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
        let view_marker = quote! {};

        if let Some(children) = children {
            let bindables =
                items_to_bind.iter().map(|ident| quote! { #ident, });

            let clonables = items_to_clone.iter().map(|ident| {
                let ident_ref = quote_spanned!(ident.span()=> &#ident);
                quote! { let #ident = ::core::clone::Clone::clone(#ident_ref); }
            });

            if bindables.len() > 0 {
                quote_spanned! {children.span()=>
                    .children({
                        #(#clonables)*

                        move |#(#bindables)*| #children #view_marker
                    })
                }
            } else {
                quote_spanned! {children.span()=>
                    .children({
                        #(#clonables)*

                        ::leptos::children::ToChildren::to_children(move || #children #view_marker)
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

    let build = quote_spanned! {node.name().span()=>
        .build()
    };

    let slot = quote_spanned! {node.span()=>
        #[allow(unused_braces)] {
            let slot = #component_name::builder()
                #(#props)*
                #(#slots)*
                #children
                #build
                #dyn_attrs;

            #[allow(unreachable_code, clippy::useless_conversion)]
            slot.into()
        },
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
