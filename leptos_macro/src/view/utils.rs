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
/// type alias), keeping the original span on both would force the
/// IDE to ask "which one?". By giving the type-namespace usage
/// (builder / check calls) a `call_site` span, only the function
/// reference remains linked to the source token, so ctrl+click
/// navigates straight to the function.
pub(crate) fn delinked_path_from_node_name(name: &NodeName) -> TokenStream {
    match name {
        NodeName::Path(expr_path) => {
            let mut new_path = expr_path.clone();
            if let Some(last) = new_path.path.segments.last_mut() {
                last.ident =
                    Ident::new(&last.ident.to_string(), Span::call_site());
            }
            quote! { #new_path }
        }
        other => quote! { #other },
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

/// Computes the `(check_fn, checked_var, check_span)` triple for a
/// prop attribute. Wraps `attr_check_info` with ident construction.
pub fn attr_check_idents(attr: &KeyedAttribute) -> (Ident, Ident, Span) {
    let (clean_prop, check_span) = attr_check_info(attr);
    let check_fn = Ident::new(&format!("__check_{}", clean_prop), check_span);
    let checked_var =
        Ident::new(&format!("__checked_{clean_prop}"), check_span);
    (check_fn, checked_var, check_span)
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

/// Returns a span covering the key–value pair of a prop assignment.
///
/// When a `value` span is available, joins it with `key`; otherwise
/// falls back to `fallback` (typically the key span itself).
pub fn key_value_span(key: Span, value: Option<Span>, fallback: Span) -> Span {
    value
        .map(|value| key.join(value).unwrap_or(key))
        .unwrap_or(fallback)
}

/// Generates the pre-check `let` statements that call companion
/// struct check methods and chain `.__pass()` for `{error}`
/// propagation.
///
/// Each entry is `(check_fn, checked_var, value, span)`.
pub(crate) fn generate_pre_check_tokens(
    checks: &[(Ident, Ident, TokenStream, Span)],
    component_path: &TokenStream,
) -> Vec<TokenStream> {
    checks
        .iter()
        .map(|(check_fn, checked_var, value, span)| {
            let pass_ident = Ident::new("__pass", *span);
            quote_spanned! {*span=>
                let #checked_var = #component_path ::#check_fn(
                    #[allow(unused_braces)] { #value }
                ).#pass_ident();
            }
        })
        .collect()
}
