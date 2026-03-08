use super::props::PropInfo;
use crate::view::{fragment_to_tokens, text_to_tokens, TagType};
use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::{quote, quote_spanned};
use rstml::node::{CustomNode, Node, NodeBlock};
use std::collections::HashMap;
use syn::{spanned::Spanned, Expr, Item, Stmt};

/// Generates the full "checked builder block" — the shared sequence
/// of helper init, presence tracking, pre-checks, builder setters,
/// and presence gate used by both component and slot code generation.
///
/// The caller provides the helper init expression and presence
/// variable name (which differ between components and slots), then
/// appends its own finalization (`.build()`, optional props, etc.)
/// after the returned block.
pub(crate) fn generate_checked_builder_block(
    helper_init: TokenStream,
    presence_ident: &Ident,
    prop_infos: &[PropInfo],
    slots: HashMap<String, Vec<TokenStream>>,
    children_arg: Option<&TokenStream>,
    children_span: Span,
) -> TokenStream {
    // Generate children pre-check and builder call.
    let (children_check_stmt, children_builder_call) =
        generate_children_check_and_builder(children_arg, children_span);

    // Collect slot names before draining, for presence tracking.
    let slot_names: Vec<String> = slots.keys().cloned().collect();
    let slot_setters = drain_slot_setters(slots);

    // Separate pre-check statements from builder setter calls.
    let prop_check_result = generate_prop_check_statements(prop_infos);
    let check_stmts = &prop_check_result.stmts;

    // Builder setter calls referencing pre-checked variables.
    let builder_setters =
        generate_builder_setters(&prop_check_result, prop_infos);

    // Presence tracking setters.
    let presence_setters = generate_presence_setters(
        prop_infos,
        &slot_names,
        children_arg.is_some(),
        presence_ident,
    );

    // Use the presence ident's span (= component/slot name span)
    // so that errors from `require_props()` and `check_missing()`
    // point to the component/slot name, not the `view!` invocation.
    let span = presence_ident.span();

    quote_spanned! {span=>
        // Obtain the helper (carries generic params).
        let __helper = #helper_init;

        // Check presence of required elements.
        let #presence_ident = __helper.presence();
        #presence_setters
        #presence_ident.require_props();

        // Pre-check prop types (independent of builder).
        #(#check_stmts)*
        #children_check_stmt

        // Initialize the props builder.
        let __props_builder = __helper.builder();

        #(#builder_setters)*
        #(#slot_setters)*
        #children_builder_call

        // Pass the typed builder instance through the presence gate.
        // When a required prop is missing, `check_missing` fails
        // (E0599) → builder becomes `{error}` → suppresses
        // TypedBuilder's confusing `.build()` error.
        let __props_builder = #presence_ident.check_missing(__props_builder);
    }
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

    let clonables = items_to_clone_to_tokens(items_to_clone);

    let children_expr = if !items_to_bind.is_empty() {
        let bindables = items_to_bind.iter().map(|ident| quote! { #ident, });
        quote_spanned! {children_span=> move |#(#bindables)*| #children }
    } else {
        quote_spanned! {children_span=>
            ::leptos::children::ToChildren::to_children(move || #children)
        }
    };

    Some(quote_spanned! {children_span=>
        {
            #(#clonables)*
            #children_expr
        }
    })
}

/// The output of `generate_prop_check_statements`.
struct CheckedPropBindings {
    /// Independent `let` bindings for pre-checking each prop value.
    pub stmts: Vec<TokenStream>,
    /// The `__checked_foo` idents for use in builder setter calls.
    pub checked_vars: Vec<Ident>,
}

/// Generates separated pre-check `let` statements for each prop.
///
/// Separating checks from builder setters ensures that when one
/// check fails (producing `{error}`), it only contaminates its own
/// variable — other checks still evaluate independently and produce
/// their own errors.
///
/// Each statement creates a local `__helper` ident with `error_span`
/// so that error messages point to the prop value, not the component
/// name.
fn generate_prop_check_statements(checks: &[PropInfo]) -> CheckedPropBindings {
    let mut stmts = Vec::with_capacity(checks.len());
    let mut checked_vars = Vec::with_capacity(checks.len());

    for info in checks {
        let span_info = &info.span_info;
        let value = &info.value;
        let span = span_info.error_span;

        let helper_local = Ident::new("__helper", span);
        let check_and_wrap = Ident::new(
            &format!("check_and_wrap_{}", span_info.stripped_name),
            span,
        );
        let checked_var =
            Ident::new(&format!("__checked_{}", span_info.stripped_name), span);

        stmts.push(quote_spanned! {span=>
            let #checked_var =
                #helper_local.#check_and_wrap(
                    #[allow(unused_braces)] { #value }
                ).extract_value();
        });
        checked_vars.push(checked_var);
    }

    CheckedPropBindings {
        stmts,
        checked_vars,
    }
}

/// Generates the children pre-check statement and builder setter
/// call, routing children through the helper for type inference.
///
/// When behavioral bounds are stripped from the props struct, the
/// TypedBuilder-generated builder no longer carries where-clause
/// constraints like `EF: Fn(T) -> N`. Passing children through
/// the helper's `check_and_wrap_children` method provides these
/// constraints (via the bounded impl block), enabling the compiler
/// to infer closure parameter types for `let:` bindings.
///
/// Returns `(check_stmt, builder_call)`, both empty when
/// `children_arg` is `None`.
fn generate_children_check_and_builder(
    children_arg: Option<&TokenStream>,
    children_span: Span,
) -> (TokenStream, TokenStream) {
    if let Some(arg) = children_arg {
        let helper_local = Ident::new("__helper", children_span);
        let check_and_wrap =
            Ident::new("check_and_wrap_children", children_span);
        let checked_var = Ident::new("__checked_children", children_span);

        let check = quote_spanned! {children_span=>
            let #checked_var =
                #helper_local.#check_and_wrap(
                    #[allow(unused_braces)] { #arg }
                ).extract_value();
        };
        let setter = quote_spanned! {children_span=>
            let __props_builder =
                __props_builder.children(#checked_var);
        };
        (check, setter)
    } else {
        (quote! {}, quote! {})
    }
}

/// Generates presence-tracking setter calls for props, slots, and
/// children.
///
/// Each non-optional prop, slot, and children gets a setter call
/// on the presence builder to transition the type-state. The
/// `var_name` ident controls the variable name used (e.g.
/// `__presence` for components, `__slot_pres` for slots).
fn generate_presence_setters(
    prop_infos: &[PropInfo],
    slot_names: &[String],
    has_children: bool,
    var_name: &Ident,
) -> TokenStream {
    let prop_setters = prop_infos.iter().map(|info| {
        let setter =
            Ident::new_raw(&info.span_info.stripped_name, Span::call_site());
        quote! { let #var_name = #var_name.#setter(); }
    });

    let slot_setters = slot_names.iter().map(|name| {
        let setter = Ident::new(name, Span::call_site());
        quote! { let #var_name = #var_name.#setter(); }
    });

    let children_setter = if has_children {
        quote! { let #var_name = #var_name.children(); }
    } else {
        quote! {}
    };

    quote! {
        #(#prop_setters)*
        #(#slot_setters)*
        #children_setter
    }
}

/// Generates builder setter calls for pre-checked prop values.
///
/// Each setter uses the `setter_span` from `PropInfo` so that
/// error messages point to the prop assignment, not the component
/// name.
fn generate_builder_setters(
    check_result: &CheckedPropBindings,
    prop_infos: &[PropInfo],
) -> Vec<TokenStream> {
    check_result
        .checked_vars
        .iter()
        .zip(prop_infos.iter())
        .map(|(checked_var, info)| {
            let setter_name = &info.setter_name;
            let span = info.setter_span;
            quote_spanned! {span=>
                #[allow(unused_braces)]
                let __props_builder = __props_builder
                    .#setter_name(#checked_var);
            }
        })
        .collect()
}

/// Consumes a slot map into builder setter calls.
///
/// Each slot entry becomes a `let __props_builder =
/// __props_builder.slot_name(value);` statement. Multi-valued
/// slots are collected into a `Vec`.
fn drain_slot_setters(
    slots: HashMap<String, Vec<TokenStream>>,
) -> Vec<TokenStream> {
    slots
        .into_iter()
        .map(|(slot, mut values)| {
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
        })
        .collect()
}

fn items_to_clone_to_tokens(
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
fn maybe_optimised_component_children(
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
                    let mac = match stmt {
                        Stmt::Macro(m) => &m.mac,
                        Stmt::Item(Item::Macro(m)) => &m.mac,
                        Stmt::Expr(Expr::Macro(m), _) => &m.mac,
                        _ => return None,
                    };
                    if !is_supported(mac) {
                        return None;
                    }
                    quote! { #block }
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
