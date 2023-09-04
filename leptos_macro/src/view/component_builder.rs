#[cfg(debug_assertions)]
use super::ident_from_tag_name;
use super::{
    client_builder::{fragment_to_tokens, TagType},
    event_from_attribute_node,
};
use proc_macro2::{Ident, TokenStream, TokenTree};
use quote::{format_ident, quote};
use rstml::node::{NodeAttribute, NodeElement};
use std::collections::HashMap;
use syn::spanned::Spanned;

pub(crate) fn component_to_tokens(
    node: &NodeElement,
    global_class: Option<&TokenTree>,
) -> TokenStream {
    let name = node.name();
    #[cfg(debug_assertions)]
    let component_name = ident_from_tag_name(node.name());
    let span = node.name().span();

    let attrs = node.attributes().iter().filter_map(|node| {
        if let NodeAttribute::Attribute(node) = node {
            Some(node)
        } else {
            None
        }
    });

    let props = attrs
        .clone()
        .filter(|attr| {
            !attr.key.to_string().starts_with("bind:")
                && !attr.key.to_string().starts_with("clone:")
                && !attr.key.to_string().starts_with("on:")
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

    let events = attrs
        .filter(|attr| attr.key.to_string().starts_with("on:"))
        .map(|attr| {
            let (event_type, handler) = event_from_attribute_node(attr, true);

            quote! {
                .on(#event_type, #handler)
            }
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
                .#slot(vec![
                    #(#values)*
                ])
            }
        } else {
            let value = &values[0];
            quote! { .#slot(#value) }
        }
    });

    let generics = &node.open_tag.generics;
    let generics = if generics.lt_token.is_some() {
        quote! { ::#generics }
    } else {
        quote! {}
    };

    #[allow(unused_mut)] // used in debug
    let mut component = quote! {
        ::leptos::component_view(
            &#name,
            ::leptos::component_props_builder(&#name #generics)
                #(#props)*
                #(#slots)*
                #children
                .build()
        )
    };

    // (Temporarily?) removed
    // See note on the function itself below.
    /* #[cfg(debug_assertions)]
    IdeTagHelper::add_component_completion(&mut component, node); */

    if events.is_empty() {
        component
    } else {
        quote! {
            #component.into_view()
            #(#events)*
        }
    }
}
