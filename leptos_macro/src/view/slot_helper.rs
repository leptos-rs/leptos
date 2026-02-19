use super::{
    convert_to_snake_case,
    utils::{
        attr_check_idents, children_span, delinked_path_from_node_name,
        extract_children_arg, generate_check_imports,
        generate_pre_check_tokens, generate_presence_setters,
        module_import_path, PropCheckInfo,
    },
};
use crate::view::utils::filter_prefixed_attrs;
use proc_macro2::{Ident, Span, TokenStream, TokenTree};
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

    // Build the module path (__SlotName) for check trait imports
    let module_path = delinked_path_from_node_name(node.name(), "__");
    let module_import_path = module_import_path(node.name(), &module_path);

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
    let mut prop_infos: Vec<PropCheckInfo> = vec![];
    let mut builder_setters: Vec<TokenStream> = vec![];
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

        let idents = attr_check_idents(attr);
        let checked_var = &idents.checked_var;

        builder_setters.push(quote_spanned! {idents.check_span=>
            let __props_builder = __props_builder
                .#attr_name(#[allow(unused_braces)] #checked_var);
        });

        prop_infos.push(PropCheckInfo { idents, value });
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

    // Generate children builder call (no pre-check, same as
    // components — children are passed directly to preserve type
    // inference).
    let children_builder_call = if let Some(ref arg) = children_arg {
        quote_spanned! {children_span=>
            let __props_builder =
                __props_builder.children(
                    #[allow(unused_braces)] { #arg }
                );
        }
    } else {
        quote! {}
    };

    // Collect slot names before draining, for presence tracking.
    let sub_slot_names: Vec<String> = slots.keys().cloned().collect();

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

    // Generate two-step pre-checks (same pattern as components).
    let check_imports =
        generate_check_imports(&prop_infos, &module_import_path);
    let pre_checks =
        generate_pre_check_tokens(&prop_infos, &module_import_path);

    // Presence tracking setters (independent of {error}).
    let slot_pres_var = Ident::new("__slot_pres", Span::call_site());
    let (presence_setters, presence_sub_slots, presence_children) =
        generate_presence_setters(
            &prop_infos,
            &sub_slot_names,
            children_arg.is_some(),
            &slot_pres_var,
        );

    let generics = &node.open_tag.generics;
    let generics = if generics.lt_token.is_some() {
        quote! { ::#generics }
    } else {
        quote! {}
    };

    let build = quote_spanned! {node.name().span()=>
        .build()
    };

    let slot = quote_spanned! {node.span()=>
        {
            #(#check_imports)*

            #(#pre_checks)*

            // Presence tracking (independent of {error})
            let __slot_pres =
                #module_path ::__presence();
            #(#presence_setters)*
            #(#presence_sub_slots)*
            #presence_children
            <_ as #module_path ::__CheckPresence>
                ::__require_props(&__slot_pres);

            // Initialize the props builder.
            let __props_builder = #module_path ::__builder #generics ();

            #(#builder_setters)*
            #(#slots)*
            #children_builder_call

            // Pass the typed builder instance through the presence gate. When a required
            // prop is missing, `__check_missing` fails (E0599) → builder becomes `{error}`
            // → suppresses TypedBuilder's confusing `.build()` error.
            let __props_builder = __slot_pres.__check_missing(__props_builder);

            let slot = __props_builder #build;
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
