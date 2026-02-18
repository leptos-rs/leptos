use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use rstml::node::{CustomNode, KeyedAttribute, Node, NodeName};
use syn::{spanned::Spanned, ExprPath};

/// Copies a `NodeName` path, replacing the last segment's span with
/// `Span::call_site()`.
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
) -> TokenStream {
    match name {
        NodeName::Path(expr_path) => {
            let mut new_path = expr_path.clone();
            if let Some(last) = new_path.path.segments.last_mut() {
                last.ident = Ident::new(
                    &last.ident.to_string(),
                    Span::call_site(),
                );
            }
            quote! { #new_path }
        }
        other => quote! { #other },
    }
}

pub fn filter_prefixed_attrs<'a, A>(
    attrs: A,
    prefix: &str,
) -> Vec<Ident>
where
    A: IntoIterator<Item = &'a KeyedAttribute> + Clone,
{
    attrs
        .into_iter()
        .filter_map(|attr| {
            attr.key
                .to_string()
                .strip_prefix(prefix)
                .map(|ident| {
                    format_ident!("{ident}", span = attr.key.span())
                })
        })
        .collect()
}

/// Handle nostrip: prefix:
/// if there strip from the name, and return true to indicate that
/// the prop should be an Option<T> and shouldn't be called on the
/// builder if None, if Some(T) then T supplied to the builder.
pub fn is_nostrip_optional_and_update_key(
    key: &mut NodeName,
) -> bool {
    let maybe_cleaned_name_and_span =
        if let NodeName::Punctuated(punct) = &key {
            if punct.len() == 2 {
                if let Some(cleaned_name) =
                    key.to_string().strip_prefix("nostrip:")
                {
                    punct.get(1).map(|segment| {
                        (cleaned_name.to_string(), segment.span())
                    })
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
    if let Some((cleaned_name, span)) = maybe_cleaned_name_and_span
    {
        *key = NodeName::Path(ExprPath {
            attrs: vec![],
            qself: None,
            path: format_ident!("{}", cleaned_name, span = span)
                .into(),
        });
        true
    } else {
        false
    }
}

/// Computes the `(clean_prop_name, check_span)` pair for a prop
/// attribute, used to generate per-prop check calls.
///
/// The clean prop name has raw identifier prefix (`r#`) stripped.
/// The span points to the value expression (or the key if there
/// is no value).
fn attr_check_info(attr: &KeyedAttribute) -> (String, Span) {
    let name = &attr.key;
    let clean_name = name.to_string().replace("r#", "");
    let check_span = attr
        .value()
        .map(|v| v.span())
        .unwrap_or_else(|| attr.key.span());
    (clean_name, check_span)
}

/// Computes the check identifiers for a prop attribute:
/// - `check_fn`: `__check_foo` at value span (for slot wrapper
///   pattern)
/// - `pass_trait`: `__Pass_foo` at `call_site()` span (for
///   component pass-trait import)
/// - `pass_method`: `__pass_foo` at value span (for component
///   pass-trait method call)
/// - `checked_var`: `__checked_foo` at value span
/// - `check_span`: value span (or key span if no value)
pub fn attr_check_idents(
    attr: &KeyedAttribute,
) -> (Ident, Ident, Ident, Ident, Span) {
    let (clean_prop, check_span) = attr_check_info(attr);
    let check_fn =
        Ident::new(&format!("__check_{}", clean_prop), check_span);
    let pass_trait = format_ident!("__Pass_{}", clean_prop);
    let pass_method =
        Ident::new(&format!("__pass_{}", clean_prop), check_span);
    let checked_var =
        Ident::new(&format!("__checked_{clean_prop}"), check_span);
    (check_fn, pass_trait, pass_method, checked_var, check_span)
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

/// Returns a span covering the key–value pair of a prop
/// assignment.
///
/// When a `value` span is available, joins it with `key`;
/// otherwise falls back to `fallback` (typically the key span
/// itself).
pub fn key_value_span(
    key: Span,
    value: Option<Span>,
    fallback: Span,
) -> Span {
    value
        .map(|value| key.join(value).unwrap_or(key))
        .unwrap_or(fallback)
}

/// Pre-check info for a single non-optional prop.
///
/// Collects the identifiers and spans needed to generate both
/// the pre-check `let` statement and the builder setter call.
pub(crate) struct PropCheckInfo {
    /// Check function ident (e.g. `__check_foo`) with value span.
    /// Used by slot pre-checks (wrapper + `__PropPass` pattern).
    pub check_fn: Ident,
    /// Pass trait ident (e.g. `__Pass_foo`) at `call_site()` span.
    /// Used by component pre-checks (import for method syntax).
    pub pass_trait: Ident,
    /// Pass method ident (e.g. `__pass_foo`) at value span.
    /// Used by component pre-checks (method call).
    pub pass_method: Ident,
    /// Checked variable ident (e.g. `__checked_foo`) with value
    /// span.
    pub checked_var: Ident,
    /// The value expression to check.
    pub value: TokenStream,
    /// Span of the value (or key if no value).
    pub check_span: Span,
}

/// Generates pre-check `let` statements for **slots** using the
/// wrapper + `.__pass()` pattern for `{error}` propagation.
pub(crate) fn generate_slot_pre_check_tokens(
    checks: &[PropCheckInfo],
    component_path: &TokenStream,
) -> Vec<TokenStream> {
    checks
        .iter()
        .map(|info| {
            let pass_ident =
                Ident::new("__pass", info.check_span);
            let check_fn = &info.check_fn;
            let checked_var = &info.checked_var;
            let value = &info.value;
            let span = info.check_span;
            quote_spanned! {span=>
                let #checked_var =
                    #component_path ::#check_fn(
                        #[allow(unused_braces)] { #value }
                    ).#pass_ident();
            }
        })
        .collect()
}

/// Generates trait imports for component pass traits. Each prop
/// gets `use Component::__Pass_foo as _;` to enable method syntax.
pub(crate) fn generate_component_pass_imports(
    checks: &[PropCheckInfo],
    component_path: &TokenStream,
) -> Vec<TokenStream> {
    checks
        .iter()
        .map(|info| {
            let pass_trait = &info.pass_trait;
            quote! {
                #[allow(unused_imports)]
                use #component_path::#pass_trait as _;
            }
        })
        .collect()
}

/// Generates pre-check `let` statements for **components** using
/// pass-trait method calls through the companion module. Each
/// call is `value.__pass_foo()`. For bounded generic props, when
/// the trait bound fails, E0599 fires with the custom
/// `on_unimplemented` message and the expression type is
/// `{error}`, suppressing downstream errors. For unbounded props,
/// the blanket pass-trait impl lets all types through.
pub(crate) fn generate_component_pre_check_tokens(
    checks: &[PropCheckInfo],
) -> Vec<TokenStream> {
    checks
        .iter()
        .map(|info| {
            let pass_method = &info.pass_method;
            let checked_var = &info.checked_var;
            let value = &info.value;
            let span = info.check_span;
            quote_spanned! {span=>
                let #checked_var = {
                    #[allow(unused_braces)]
                    { #value }
                }.#pass_method();
            }
        })
        .collect()
}
