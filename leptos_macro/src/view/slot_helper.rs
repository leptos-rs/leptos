use super::{
    convert_to_snake_case,
    utils::{
        children_span, delinked_path_from_node_name, extract_children_arg,
        generate_checked_builder_block, prop_span_info, turbofish_generics,
        PropInfo,
    },
};
use crate::view::utils::filter_prefixed_attrs;
use proc_macro2::{Ident, TokenStream, TokenTree};
use quote::{quote, quote_spanned};
use rstml::node::{CustomNode, KeyedAttribute, NodeAttribute, NodeElement};
use std::collections::{HashMap, HashSet};
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

    // Build the struct path (SlotName) for inherent method calls.
    // Uses call_site() span so IDE ctrl+click goes to the function,
    // not the companion module.
    let struct_path = delinked_path_from_node_name(node.name(), "");

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

    // Collect pre-check info for each non-optional prop.
    let mut prop_infos: Vec<PropInfo> = vec![];
    let mut seen_prop_names = HashSet::new();
    for attr in attrs.iter().filter(|attr| {
        !attr.key.to_string().starts_with("let:")
            && !attr.key.to_string().starts_with("clone:")
            && !attr.key.to_string().starts_with("attr:")
    }) {
        let attr_name = &attr.key;

        let name_str = attr_name.to_string();
        if !seen_prop_names.insert(name_str.clone()) {
            proc_macro_error2::emit_error!(
                attr_name.span(),
                "duplicate prop `{}` — each prop can only be set once",
                name_str
            );
            continue;
        }

        let value = attr
            .value()
            .map(|v| {
                quote! { #v }
            })
            .unwrap_or_else(|| quote! { #attr_name });

        let span_info = prop_span_info(attr);
        let setter_span = span_info.check_span;

        prop_infos.push(PropInfo {
            span_info,
            value,
            setter_name: quote! { #attr_name },
            setter_span,
        });
    }

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
    let helper_init = quote! { #struct_path #generics ::__slot() };
    // Use `name_span` so that where-clause failures on
    // `require_props()` and E0599 on `check_missing()` point
    // to the slot name, not the entire `view!` invocation.
    // Use `__slot_pres` (not `__presence`) to avoid shadowing the
    // parent component's `__presence` when a slot is nested inside
    // a component's view.
    let presence_ident = Ident::new("__slot_pres", name_span);

    let builder_block = generate_checked_builder_block(
        helper_init,
        &presence_ident,
        &prop_infos,
        &mut slots,
        children_arg.as_ref(),
        children_span,
    );

    let build = quote_spanned! {node.name().span()=>
        .build()
    };

    let slot = quote_spanned! {node.span()=>
        {
            #builder_block

            let slot = __props_builder #build;
            let slot = slot #dyn_attrs;

            #[allow(unreachable_code, clippy::useless_conversion)]
            slot.into()
        },
    };

    // We need to move "allow" out of "quote_spanned" because it breaks hovering
    // in rust-analyzer
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
