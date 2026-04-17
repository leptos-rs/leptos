use super::span::PropSpanInfo;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};
use rstml::node::{KeyedAttribute, NodeName};
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

/// Converts node generics to turbofish syntax (`::<A, B>`).
///
/// Returns empty tokens when there are no generics.
pub(crate) fn turbofish_generics(generics: &syn::Generics) -> TokenStream {
    if generics.lt_token.is_some() {
        quote! { ::#generics }
    } else {
        quote! {}
    }
}

/// Info for a single non-optional prop.
///
/// Collects the span info, value expression, and builder setter
/// details needed to generate pre-check statements, presence
/// tracking, and builder setter calls.
pub(crate) struct PropInfo {
    /// Span and name info for this prop.
    pub span_info: PropSpanInfo,
    /// The value expression to check.
    pub value: TokenStream,
    /// The prop name tokens for the builder setter call.
    pub setter_name: TokenStream,
    /// The span for the builder setter call.
    ///
    /// Components use `key_value_span()` (joined key+value),
    /// slots use `error_span` (value only).
    pub setter_span: Span,
}
