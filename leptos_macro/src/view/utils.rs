use proc_macro2::{Ident, Span};
use quote::format_ident;
use rstml::node::{CustomNode, KeyedAttribute, Node, NodeName};
use syn::{spanned::Spanned, ExprPath};

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
/// the prop should be an Option<T> and shouldn't be called on the builder if None,
/// if Some(T) then T supplied to the builder.
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

// struct AttrCheck {
//     check_fn: Ident,
//     span: Span,
// }

/// Computes the `(check_fn_ident, check_span)` pair for a prop
/// attribute, used to generate per-prop `__check_*()` calls.
///
/// The check function name is `__check_{prop_name}` (with raw
/// identifier prefix stripped). The span points to the value
/// expression (or the key if there is no value).
pub fn attr_check_info(attr: &KeyedAttribute) -> (Ident, Span) {
    let name = &attr.key;
    let check_fn_name = name.to_string().replace("r#", "");
    let check_fn = format_ident!("__check_{}", check_fn_name);
    let check_span = attr
        .value()
        .map(|v| v.span())
        .unwrap_or_else(|| attr.key.span());
    (check_fn, check_span)
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

pub fn key_value_span(key: Span, value: Option<Span>, fallback: Span) -> Span {
    value
        .map(|value| key.join(value.span()).unwrap_or(key.span()))
        .unwrap_or(fallback)
}

/// Converts a `NodeName` to an `Ident` with the given span.
///
/// Uses `NodeName`'s `Display` impl, which preserves raw identifier
/// prefixes (e.g. `r#type`), then reconstructs the `Ident` with the
/// target span.
pub fn node_name_to_ident_with_span(name: &NodeName, span: Span) -> Ident {
    let s = name.to_string();
    if let Some(raw) = s.strip_prefix("r#") {
        Ident::new_raw(raw, span)
    } else {
        Ident::new(&s, span)
    }
}
