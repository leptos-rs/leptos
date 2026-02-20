//! View-side utility helpers for ident/span manipulation.
//!
//! These helpers operate on `rstml` nodes and are used during view
//! macro expansion. In contrast, `crate::util` contains type-analysis
//! and companion-module generation logic shared by the `#[component]`
//! and `#[slot]` proc macros.

use crate::view::{
    component_builder::{
        items_to_clone_to_tokens, maybe_optimised_component_children,
    },
    fragment_to_tokens, TagType,
};
use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::{format_ident, quote, quote_spanned};
use rstml::node::{CustomNode, KeyedAttribute, Node, NodeName};
use std::collections::HashMap;
use syn::{spanned::Spanned, ExprPath};

/// Copies a `NodeName` path, optionally prepending `prefix` to
/// the last segment's identifier, and replacing its span with
/// `Span::call_site()`.
///
/// Pass `""` for components (delinks the span without renaming)
/// or `"__"` for slots (produces the `__SlotName` module path).
///
/// IDEs resolve ctrl+click targets by matching source spans to
/// expanded spans. Because a component name exists in both the value
/// namespace (the function) and the type namespace (the companion
/// module), keeping the original span on both would force the
/// IDE to ask "which one?". By giving the type-namespace usage
/// (builder / check calls) a `call_site` span, only the function
/// reference remains linked to the source token, so ctrl+click
/// navigates straight to the function.
pub(crate) fn delinked_path_from_node_name(
    name: &NodeName,
    prefix: &str,
) -> TokenStream {
    match name {
        NodeName::Path(expr_path) => {
            let mut new_path = expr_path.clone();
            if let Some(last) = new_path.path.segments.last_mut() {
                last.ident = Ident::new(
                    &format!("{prefix}{}", last.ident),
                    Span::call_site(),
                );
            }
            quote! { #new_path }
        }
        other => {
            if prefix.is_empty() {
                quote! { #other }
            } else {
                let s = other.to_string();
                let module_ident =
                    format_ident!("{prefix}{s}", span = Span::call_site());
                quote! { #module_ident }
            }
        }
    }
}

pub fn filter_prefixed_attrs<'a, A>(attrs: A, prefix: &str) -> Vec<Ident>
where
    A: IntoIterator<Item = &'a KeyedAttribute> + Clone,
{
    attrs
        .into_iter()
        .filter_map(|attr| {
            attr.key
                .to_string()
                .strip_prefix(prefix)
                .map(|ident| format_ident!("{ident}", span = attr.key.span()))
        })
        .collect()
}

/// Handle nostrip: prefix:
/// if there strip from the name, and return true to indicate that
/// the prop should be an Option<T> and shouldn't be called on the
/// builder if None, if Some(T) then T supplied to the builder.
pub fn is_nostrip_optional_and_update_key(key: &mut NodeName) -> bool {
    let maybe_cleaned_name_and_span = if let NodeName::Punctuated(punct) = &key
    {
        if punct.len() == 2 {
            if let Some(cleaned_name) = key.to_string().strip_prefix("nostrip:")
            {
                punct
                    .get(1)
                    .map(|segment| (cleaned_name.to_string(), segment.span()))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };
    if let Some((cleaned_name, span)) = maybe_cleaned_name_and_span {
        *key = NodeName::Path(ExprPath {
            attrs: vec![],
            qself: None,
            path: format_ident!("{}", cleaned_name, span = span).into(),
        });
        true
    } else {
        false
    }
}

/// Computes the check identifiers for a prop attribute:
/// - `check_trait`: `__Check_foo` at `call_site()` (UFCS path +
///   import)
/// - `check_method`: `__check_foo` at value span (UFCS method)
/// - `pass_method`: `__pass_foo` at value span (method call)
/// - `checked_var`: `__checked_foo` at value span
/// - `check_span`: value span (or key span if no value)
pub fn attr_check_idents(attr: &KeyedAttribute) -> PropCheckIdents {
    let name = &attr.key;
    let clean_name = name.to_string().replace("r#", "");
    let check_span = attr
        .value()
        .map(|v| v.span())
        .unwrap_or_else(|| attr.key.span());
    PropCheckIdents {
        check_trait: format_ident!("__Check_{}", clean_name),
        check_method: Ident::new(
            &format!("__check_{}", clean_name),
            check_span,
        ),
        pass_method: Ident::new(&format!("__pass_{}", clean_name), check_span),
        checked_var: Ident::new(&format!("__checked_{clean_name}"), check_span),
        check_span,
        clean_name,
    }
}

/// All identifiers needed for a prop's pre-check.
pub struct PropCheckIdents {
    pub check_trait: Ident,
    pub check_method: Ident,
    pub pass_method: Ident,
    pub checked_var: Ident,
    pub check_span: Span,
    /// The clean prop name (raw identifier prefix stripped).
    pub clean_name: String,
}

/// Computes a span covering all children of a node.
///
/// Joins the span of the first child to the span of the last
/// child (on nightly). Falls back to `fallback` (typically the
/// component/slot name span) when there are no children or join
/// is unavailable.
pub fn children_span<C: CustomNode>(
    children: &[Node<C>],
    fallback: Span,
) -> Span {
    match (children.first(), children.last()) {
        (Some(first), Some(last)) => first
            .span()
            .join(last.span())
            .unwrap_or_else(|| first.span()),
        _ => fallback,
    }
}

/// Returns a span covering the key-value pair of a prop
/// assignment.
///
/// When a `value` span is available, joins it with `key`;
/// otherwise falls back to `fallback` (typically the key span
/// itself).
pub fn key_value_span(key: Span, value: Option<Span>, fallback: Span) -> Span {
    value
        .map(|value| key.join(value).unwrap_or(key))
        .unwrap_or(fallback)
}

/// Pre-check info for a single non-optional prop.
///
/// Collects the identifiers and spans needed to generate both
/// the pre-check `let` statement and the builder setter call.
pub(crate) struct PropCheckInfo {
    /// All check/pass identifiers for this prop.
    pub idents: PropCheckIdents,
    /// The value expression to check.
    pub value: TokenStream,
}

/// Generates trait imports for check traits. Each prop gets
/// `use Module::__Check_foo as _;` to enable method syntax for
/// the `__pass_foo()` call (`{error}` propagation step).
pub(crate) fn generate_check_imports(
    checks: &[PropCheckInfo],
    module_path: &TokenStream,
) -> Vec<TokenStream> {
    checks
        .iter()
        .map(|info| {
            let check_trait = &info.idents.check_trait;
            quote! {
                #[allow(unused_imports)]
                use #module_path::#check_trait as _;
            }
        })
        .collect()
}

/// For trait imports from the companion module, we may need
/// `self::Module::__Check_foo` for single-segment paths to
/// disambiguate from glob-imported traits with the same name.
///
/// Used by both `component_builder.rs` and `slot_helper.rs`.
pub(crate) fn module_import_path(
    name: &NodeName,
    module_path: &TokenStream,
) -> TokenStream {
    match name {
        NodeName::Path(expr_path) if expr_path.path.segments.len() == 1 => {
            quote! { self::#module_path }
        }
        _ => module_path.clone(),
    }
}

/// Generates two-step pre-check statements for each prop:
///
/// 1. UFCS check: `<_ as Module::__Check_foo>::__check_foo(&v)`
///    — E0277 with `on_unimplemented` (works for all types
///    including closures).
/// 2. Method call: `let __checked = v.__pass_foo()`
///    — E0599 produces `{error}` type, suppressing downstream
///    `__require_props` and `__check_missing` errors.
///
/// For unbounded/concrete props, both traits have blanket impls
/// so both calls succeed unconditionally.
///
/// Used by components.  Slots use `generate_struct_pre_check_tokens`
/// (inherent methods on the struct path instead of UFCS).
pub(crate) fn generate_pre_check_tokens(
    checks: &[PropCheckInfo],
    module_path: &TokenStream,
) -> Vec<TokenStream> {
    checks
        .iter()
        .map(|info| {
            let idents = &info.idents;
            let check_trait = &idents.check_trait;
            let check_method = &idents.check_method;
            let pass_method = &idents.pass_method;
            let checked_var = &idents.checked_var;
            let value = &info.value;
            let span = idents.check_span;
            let value_var =
                Ident::new(&format!("__value_{}", idents.clean_name), span);
            quote_spanned! {span=>
                #[allow(unused_braces)]
                let #value_var = { #value };
                <_ as #module_path::#check_trait>::#check_method(
                    &#value_var);
                let #checked_var = #value_var.#pass_method();
            }
        })
        .collect()
}

/// Generates helper-based pre-check statements for each prop (used
/// by slots).
///
/// 1. `helper.__check_foo(&v)` — method with trait bound → E0277
///    with `on_unimplemented`.
/// 2. `let __checked = helper.__wrap_foo(v).__unwrap()` — bounded
///    inherent `__unwrap()` → E0599 → `{error}` type.
///
/// Unlike `generate_pre_check_tokens` (which uses UFCS + trait
/// imports), this goes through methods on the `__Helper` struct
/// returned by `SlotName::__slot()`. Because the helper carries
/// the slot's generic params, all calls share one set of type
/// variables (constrained by the builder chain). And because the
/// entry point is the slot struct's `__slot()` inherent method,
/// renamed imports (`use path::Slot as Alias`) work.
pub(crate) fn generate_helper_pre_check_tokens(
    checks: &[PropCheckInfo],
    helper_var: &Ident,
) -> Vec<TokenStream> {
    checks
        .iter()
        .map(|info| {
            let idents = &info.idents;
            let check_method = &idents.check_method;
            let checked_var = &idents.checked_var;
            let value = &info.value;
            let span = idents.check_span;
            let value_var =
                Ident::new(&format!("__value_{}", idents.clean_name), span);
            let wrap_method =
                Ident::new(&format!("__wrap_{}", idents.clean_name), span);
            quote_spanned! {span=>
                #[allow(unused_braces)]
                let #value_var = { #value };
                #helper_var.#check_method(&#value_var);
                let #checked_var =
                    #helper_var.#wrap_method(#value_var)
                        .__unwrap();
            }
        })
        .collect()
}

/// Generates presence-tracking setter calls for props, slots, and
/// children.
///
/// Each non-optional prop, slot, and children gets a setter call
/// on the presence builder to transition the type-state. The
/// `var_name` ident controls the variable name used (e.g.
/// `__presence` for components, `__slot_pres` for slots).
///
/// Returns `(prop_setters, slot_setters, children_setter)`.
pub(crate) fn generate_presence_setters(
    prop_infos: &[PropCheckInfo],
    slot_names: &[String],
    has_children: bool,
    var_name: &Ident,
) -> (Vec<TokenStream>, Vec<TokenStream>, TokenStream) {
    let prop_setters: Vec<TokenStream> = prop_infos
        .iter()
        .map(|info| {
            let setter =
                Ident::new_raw(&info.idents.clean_name, Span::call_site());
            quote! { let #var_name = #var_name.#setter(); }
        })
        .collect();

    let slot_setters: Vec<TokenStream> = slot_names
        .iter()
        .map(|name| {
            let setter = Ident::new(name, Span::call_site());
            quote! { let #var_name = #var_name.#setter(); }
        })
        .collect();

    let children_setter = if has_children {
        quote! { let #var_name = #var_name.children(); }
    } else {
        quote! {}
    };

    (prop_setters, slot_setters, children_setter)
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

    if !items_to_bind.is_empty() {
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
