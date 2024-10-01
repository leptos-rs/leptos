use proc_macro2::Ident;
use quote::format_ident;
use rstml::node::KeyedAttribute;
use syn::spanned::Spanned;

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
