use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};
use rstml::node::{CustomNode, KeyedAttribute, Node, NodeName};
use syn::spanned::Spanned;

/// Copies a `NodeName` path, optionally prepending `prefix` to the last
/// segment's identifier, and replacing its span with `Span::call_site()`.
///
/// Pass `prefix`
/// - `""` for components (delinks the span without renaming) or
/// - `"__"` for slots (produces the `__SlotName` module path).
///
/// IDEs resolve ctrl+click targets by matching source spans to expanded spans.
/// Because a component name exists in both the value namespace (the function)
/// and the type namespace (the companion module), keeping the original span on
/// both would force the IDE to ask "which one?". By giving the type-namespace
/// usage (builder / check calls) a `call_site` span, only the function
/// reference remains linked to the source token, so ctrl+click navigates
/// straight to the function.
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

/// Computes the span info for a prop attribute:
/// - `error_span`: value span (or key span if no value)
/// - `stripped_name`: prop name with `r#` prefix stripped
pub fn prop_span_info(attr: &KeyedAttribute) -> PropSpanInfo {
    let name = &attr.key;
    let stripped_name = name.to_string().replace("r#", "");
    let error_span = attr
        .value()
        .map(|v| v.span())
        .unwrap_or_else(|| attr.key.span());
    PropSpanInfo {
        error_span,
        stripped_name,
    }
}

/// Span and name info for a prop's pre-check and builder setter.
pub struct PropSpanInfo {
    /// The span to use for error reporting (value span, or key span
    /// for short-form props).
    pub error_span: Span,
    /// The clean prop name (raw identifier prefix stripped).
    pub stripped_name: String,
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
