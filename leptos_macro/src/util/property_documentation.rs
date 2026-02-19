use crate::util::documentation::Docs;
use crate::util::{is_option, unwrap_option};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::Type;

/// Controls the output format of prop documentation.
#[derive(Clone, Copy)]
pub(crate) enum PropDocumentationStyle {
    /// Markdown list item (used in component/slot doc comments).
    List,
    /// Inline builder-setter documentation.
    Inline,
}

/// Common fields needed to generate documentation for a single prop.
pub(crate) struct PropDocumentationInput<'a> {
    pub docs: &'a Docs,
    pub name: &'a Ident,
    pub ty: &'a Type,
    /// Whether the prop is optional (for section categorization).
    pub is_optional: bool,
    /// Raw `#[prop(optional)]` flag (for `Option<T>` unwrapping).
    pub optional: bool,
    pub strip_option: bool,
    pub into: bool,
}

/// Generates a documentation token stream for a single prop.
pub(crate) fn prop_to_doc(
    input: &PropDocumentationInput,
    style: PropDocumentationStyle,
) -> TokenStream {
    let ty = if (input.optional || input.strip_option) && is_option(input.ty) {
        unwrap_option(input.ty)
    } else {
        input.ty.to_owned()
    };

    let type_item: syn::Item = syn::parse_quote! {
        type SomeType = #ty;
    };

    let file = syn::File {
        shebang: None,
        attrs: vec![],
        items: vec![type_item],
    };

    let pretty_ty = prettyplease::unparse(&file);
    let pretty_ty = &pretty_ty[16..&pretty_ty.len() - 2];

    let name = input.name;

    match style {
        PropDocumentationStyle::List => {
            let arg_ty_doc = syn::LitStr::new(
                &if !input.into {
                    format!(" - **{}**: [`{pretty_ty}`]", quote!(#name))
                } else {
                    format!(
                        " - **{}**: [`impl Into<{pretty_ty}>`]({pretty_ty})",
                        quote!(#name),
                    )
                },
                name.span(),
            );

            let arg_user_docs = input.docs.padded();

            quote! {
                #[doc = #arg_ty_doc]
                #arg_user_docs
            }
        }
        PropDocumentationStyle::Inline => {
            let arg_ty_doc = syn::LitStr::new(
                &if !input.into {
                    format!(
                        "**{}**: [`{}`]{}",
                        quote!(#name),
                        pretty_ty,
                        input.docs.typed_builder()
                    )
                } else {
                    format!(
                        "**{}**: `impl`[`Into<{}>`]{}",
                        quote!(#name),
                        pretty_ty,
                        input.docs.typed_builder()
                    )
                },
                name.span(),
            );

            quote! {
                #[builder(setter(doc = #arg_ty_doc))]
            }
        }
    }
}

/// Generates grouped prop documentation with required and optional sections.
pub(crate) fn generate_prop_documentation(
    props: &[PropDocumentationInput],
) -> TokenStream {
    let required_prop_docs = props
        .iter()
        .filter(|p| !p.is_optional)
        .map(|p| prop_to_doc(p, PropDocumentationStyle::List))
        .collect::<TokenStream>();

    let optional_prop_docs = props
        .iter()
        .filter(|p| p.is_optional)
        .map(|p| prop_to_doc(p, PropDocumentationStyle::List))
        .collect::<TokenStream>();

    let required_prop_docs = if !required_prop_docs.is_empty() {
        quote! {
            #[doc = " # Required Props"]
            #required_prop_docs
        }
    } else {
        quote! {}
    };

    let optional_prop_docs = if !optional_prop_docs.is_empty() {
        quote! {
            #[doc = " # Optional Props"]
            #optional_prop_docs
        }
    } else {
        quote! {}
    };

    quote! {
        #required_prop_docs
        #optional_prop_docs
    }
}
