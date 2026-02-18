use super::{
    fragment_to_tokens,
    utils::{
        attr_check_idents, children_span, generate_pre_check_tokens,
        is_nostrip_optional_and_update_key, node_name_to_ident_with_span,
    },
    TagType,
};
use crate::view::{
    attribute_absolute, text_to_tokens,
    utils::{filter_prefixed_attrs, key_value_span},
};
use proc_macro2::{Ident, Span, TokenStream, TokenTree};
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

    // Capture component name tokens and span before mutable borrows
    let name_span = node.name().span();
    let component_path: TokenStream = {
        let n = node.name();
        quote! { #n }
    };

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

    // Collect pre-check info and builder setter info.
    //
    // For each non-optional prop we:
    // 1. Pre-check the value via companion struct method (E0277
    //    with custom `on_unimplemented` for bounded generic props).
    //    The check returns a wrapper type.
    // 2. Call `.__pass()` on the wrapper — for bounded generic
    //    props, this produces E0599 → `{error}`, suppressing all
    //    downstream errors (including __check_missing).
    // 3. Pass the result to the builder setter.
    let mut pre_check_info: Vec<(
        Ident,       // check_fn ident with check_span
        Ident,       // checked_var ident with check_span
        TokenStream, // value expression
        Span,        // check_span (value's span)
    )> = vec![];
    let mut setter_info: Vec<(
        Ident, // checked_var ident with check_span
        Ident, // setter_name (with value span for into props)
        Span,  // key_value_span (for builder setter)
    )> = vec![];
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

        let key_value_span = key_value_span(
            attr.key.span(),
            attr.value().map(|it| it.span()),
            name.span(),
        );

        // Create a setter name ident that carries the value's span.
        // For `into` props, E0277 points to the method name, so
        // this ensures the error points to the value expression.
        let setter_name = node_name_to_ident_with_span(
            name,
            attr.value()
                .map(|v| v.span())
                .unwrap_or_else(|| name.span()),
        );

        if optional {
            optional_props.push(quote_spanned! {key_value_span=>
                props.#setter_name = { #value }.map(::leptos::prelude::IntoReactiveValue::into_reactive_value);
            })
        } else {
            let (check_fn_spanned, checked_var, check_span) =
                attr_check_idents(attr);

            pre_check_info.push((
                check_fn_spanned,
                checked_var.clone(),
                value,
                check_span,
            ));
            setter_info.push((checked_var, setter_name, key_value_span));
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

    // Generate children builder call (no checker needed —
    // children are always concrete types).
    let children_builder_call = if let Some(ref arg) = children_arg {
        quote_spanned! {children_span=>
            #[allow(unused_braces)]
            let __props_builder = __props_builder.children({ #arg });
        }
    } else {
        quote! {}
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

        quote_spanned! {span=>
            let __props_builder = __props_builder.#slot(#value);
        }
    });

    let generics = &node.open_tag.generics;
    let generics = if generics.lt_token.is_some() {
        quote! { ::#generics }
    } else {
        quote! {}
    };

    // Pre-check calls via companion struct.
    // For bounded generic props, E0277 fires with custom
    // `on_unimplemented` message. The check returns a wrapper;
    // calling `.__pass()` on it produces E0599 → `{error}` when
    // bounds fail, suppressing all downstream errors.
    let pre_checks =
        generate_pre_check_tokens(&pre_check_info, &component_path);

    // Builder setter calls using pre-checked values.
    let builder_setters: Vec<TokenStream> = setter_info
        .iter()
        .map(|(checked_var, setter_name, kv_span)| {
            quote_spanned! {*kv_span=>
                let __props_builder = __props_builder
                    .#setter_name(#checked_var);
            }
        })
        .collect();

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
            // Import __PropPass for method call syntax on
            // check wrappers (enables .__pass() calls).
            #[allow(unused_imports)]
            use ::leptos::component::__PropPass as _;
            ::leptos::component::component_view(
                #[allow(clippy::needless_borrows_for_generic_args)]
                &#component_path,
                {
                    // Pre-checks (E0277 with custom message
                    // for bounded generic props, then
                    // .__pass() for {error} propagation)
                    #(#pre_checks)*
                    // Build props via companion struct
                    let __props_builder =
                        #component_path ::builder #generics ();
                    #(#builder_setters)*
                    #(#slots)*
                    #children_builder_call
                    let __props_builder =
                        __props_builder.__check_missing();
                    let #props_mut #props_ident =
                        __props_builder.build();
                    #(#optional_props)*
                    #props_ident
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

/// Extracts the children argument expression for a component or
/// slot, trying the optimised path first, then falling back to
/// `fragment_to_tokens` wrapped with bindables and clonables.
///
/// Returns `None` when there are no children or when
/// `fragment_to_tokens` produces nothing.
pub(crate) fn extract_children_arg(
    children: &mut [Node<impl CustomNode>],
    slots: &mut HashMap<String, Vec<TokenStream>>,
    items_to_bind: &[TokenStream],
    items_to_clone: &[Ident],
    children_span: Span,
    global_class: Option<&TokenTree>,
    disable_inert_html: bool,
) -> Option<TokenStream> {
    if children.is_empty() {
        return None;
    }

    if let Some(children_arg) = maybe_optimised_component_children(
        children,
        items_to_bind,
        items_to_clone,
    ) {
        return Some(children_arg);
    }

    let children = fragment_to_tokens(
        children,
        TagType::Unknown,
        Some(slots),
        global_class,
        None,
        disable_inert_html,
    );

    let Some(children) = children else {
        return None;
    };

    let bindables = items_to_bind.iter().map(|ident| quote! { #ident, });

    let clonables = items_to_clone_to_tokens(items_to_clone);

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
}
