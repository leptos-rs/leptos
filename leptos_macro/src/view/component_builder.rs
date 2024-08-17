#[cfg(debug_assertions)]
use super::ident_from_tag_name;
use super::{
    client_builder::{fragment_to_tokens, TagType},
    event_from_attribute_node,
};
use crate::view::directive_call_from_attribute_node;
use proc_macro2::{Ident, TokenStream, TokenTree};
use quote::{format_ident, quote, quote_spanned, ToTokens};
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

    let attrs = node.attributes().iter().filter_map(|node| {
        if let NodeAttribute::Attribute(node) = node {
            Some(node)
        } else {
            None
        }
    });

    let spread_bindings = node.attributes().iter().filter_map(|node| {
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
                ) => Some(
                    quote! { .dyn_bindings(#[allow(unused_brace)] {#end}) },
                ),
                _ => None,
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
                && !attr.key.to_string().starts_with("on:")
                && !attr.key.to_string().starts_with("attr:")
                && !attr.key.to_string().starts_with("use:")
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
                .#name(#[allow(unused_braces)] { #value })
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

    let events = attrs
        .clone()
        .filter(|attr| attr.key.to_string().starts_with("on:"))
        .map(|attr| {
            let (event_type, handler) = event_from_attribute_node(attr, true);
            // HACK(chrisp60): rstml and leptos has a different definition on attribute keys.
            // This retains precise span information for the "on" in "on:some_event_name".
            //
            // A similar hack is done in `event_from_attribute_node` to retain the precise
            // event name span.
            let on = attr.key.to_token_stream().into_iter().next();

            quote! {
                .#on(#event_type, #handler)
            }
        })
        .collect::<Vec<_>>();

    let directives = attrs
        .clone()
        .filter_map(|attr| {
            attr.key
                .to_string()
                .strip_prefix("use:")
                .map(|ident| directive_call_from_attribute_node(attr, ident))
        })
        .collect::<Vec<_>>();

    let events_and_directives =
        events.into_iter().chain(directives).collect::<Vec<_>>();

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
            true,
            TagType::Unknown,
            Some(&mut slots),
            global_class,
            None,
        );

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

        if let Some(children) = children {
            let bindables =
                items_to_bind.iter().map(|ident| quote! { #ident, });

            let clonables = items_to_clone.iter().map(|ident| {
                quote! { let #ident = ::core::clone::Clone::clone(&#ident); }
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

                        ::leptos::ToChildren::to_children(move || #children #view_marker)
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
            quote! {
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

    #[allow(unused_mut)] // used in debug
    let mut component = quote! {
        ::leptos::component_view(
            #[allow(clippy::needless_borrows_for_generic_args)]
            &#name,
            ::leptos::component_props_builder(&#name #generics)
                #(#props)*
                #(#slots)*
                #children
                .build()
                #dyn_attrs
                #(#spread_bindings)*
        )
    };

    // (Temporarily?) removed
    // See note on the function itself below.
    /* #[cfg(debug_assertions)]
    IdeTagHelper::add_component_completion(&mut component, node); */

    if events_and_directives.is_empty() {
        component
    } else {
        quote! {
            ::leptos::IntoView::into_view(#[allow(unused_braces)] {#component})
            #(#events_and_directives)*
        }
    }
}
