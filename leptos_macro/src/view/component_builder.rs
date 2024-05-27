use super::{event_to_tokens, fragment_to_tokens, TagType};
use crate::view::{
    attribute_absolute, attribute_to_tokens, attribute_value,
    event_type_and_handler,
};
use proc_macro2::{Ident, TokenStream, TokenTree};
use quote::{format_ident, quote, quote_spanned};
use rstml::node::{
    NodeAttribute, NodeBlock, NodeElement, NodeName, NodeNameFragment,
};
use std::collections::HashMap;
use syn::{spanned::Spanned, Expr, ExprRange, RangeLimits, Stmt};

pub(crate) fn component_to_tokens(
    node: &NodeElement,
    global_class: Option<&TokenTree>,
) -> TokenStream {
    let name = node.name();
    #[cfg(debug_assertions)]
    let component_name = ident_from_tag_name(node.name());

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
        .enumerate()
        .filter(|(idx, attr)| {
            idx < &spread_marker && {
                let attr_key = attr.key.to_string();
                !attr_key.starts_with("let:")
                    && !attr_key.starts_with("clone:")
                    && !attr_key.starts_with("class:")
                    && !attr_key.starts_with("style:")
                    && !attr_key.starts_with("attr:")
                    && !attr_key.starts_with("prop:")
                    && !attr_key.starts_with("on:")
                    && !attr_key.starts_with("use:")
            }
        })
        .map(|(_, attr)| {
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
            use rstml::node::NodeBlock;
            use syn::{Expr, ExprRange, RangeLimits, Stmt};

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
        quote! {
            .add_any_attr((#(#spreads,)*))
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
    } else {
        let children = fragment_to_tokens(
            &node.children,
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
                let ident_ref = quote_spanned!(ident.span()=> &#ident);
                quote! { let #ident = ::core::clone::Clone::clone(#ident_ref); }
            });

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

    let name_ref = quote_spanned! {name.span()=>
        &#name
    };

    let build = quote_spanned! {name.span()=>
        .build()
    };

    let component_props_builder = quote_spanned! {name.span()=>
        ::leptos::component::component_props_builder(#name_ref #generics)
    };

    #[allow(unused_mut)] // used in debug
    let mut component = quote_spanned! {node.span()=>
        {
            #[allow(unreachable_code)]
            ::leptos::component::component_view(
                #[allow(clippy::needless_borrows_for_generic_args)]
                #name_ref,
                #component_props_builder
                    #(#props)*
                    #(#slots)*
                    #children
                    #build
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

#[cfg(debug_assertions)]
fn ident_from_tag_name(tag_name: &NodeName) -> Ident {
    match tag_name {
        NodeName::Path(path) => path
            .path
            .segments
            .iter()
            .last()
            .map(|segment| segment.ident.clone())
            .expect("element needs to have a name"),
        NodeName::Block(_) => {
            let span = tag_name.span();
            proc_macro_error::emit_error!(
                span,
                "blocks not allowed in tag-name position"
            );
            Ident::new("", span)
        }
        _ => Ident::new(
            &tag_name.to_string().replace(['-', ':'], "_"),
            tag_name.span(),
        ),
    }
}
