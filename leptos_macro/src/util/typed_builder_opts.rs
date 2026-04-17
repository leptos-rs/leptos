//! Options for generating `#[builder(...)]` and `#[serde(...)]`
//! attributes on prop fields.
//!
//! [`TypedBuilderOpts`] is constructed via [`TypedBuilderOpts::from_prop`]
//! and consumed through its [`ToTokens`] impl (for builder attributes) or
//! [`TypedBuilderOpts::to_serde_tokens`] (for serialization attributes).

use crate::util::{type_analysis, unwrap_option, PropLike};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::Type;

/// Options for generating `#[builder(...)]` attributes on prop fields.
///
/// Used by both components and slots to produce the correct
/// TypedBuilder annotations for each prop.
pub(crate) struct TypedBuilderOpts<'a> {
    default: bool,
    default_with_value: Option<syn::Expr>,
    strip_option: bool,
    into: bool,
    ty: &'a Type,
}

impl<'a> TypedBuilderOpts<'a> {
    /// Computes the `TypedBuilderOpts` for a prop field from its
    /// `PropLike` implementation.
    ///
    /// Used by both components and slots.
    pub(crate) fn from_prop<P: PropLike>(prop: &'a P) -> Self {
        let ty = prop.ty();
        Self {
            default: prop.is_optional() && prop.default().is_none(),
            default_with_value: prop.default().cloned(),
            strip_option: prop.has_strip_option_flag()
                || prop.has_optional_flag() && type_analysis::is_option(ty),
            into: prop.has_into_flag(),
            ty,
        }
    }

    /// Generates `#[serde(...)]` attributes matching the builder
    /// defaults. Only used by component props serialization.
    pub fn to_serde_tokens(&self) -> TokenStream {
        let default = if let Some(v) = &self.default_with_value {
            let v = v.to_token_stream().to_string();
            quote! { default=#v, }
        } else if self.default {
            quote! { default, }
        } else {
            quote! {}
        };

        if !default.is_empty() {
            quote! { #[serde(#default)] }
        } else {
            quote! {}
        }
    }
}

impl ToTokens for TypedBuilderOpts<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let default = if let Some(v) = &self.default_with_value {
            let v = v.to_token_stream().to_string();
            quote! { default_code=#v, }
        } else if self.default {
            quote! { default, }
        } else {
            quote! {}
        };

        // If self.strip_option && self.into, then the strip_option
        // will be represented as part of the transform closure.
        let strip_option = if self.strip_option && !self.into {
            quote! { strip_option, }
        } else {
            quote! {}
        };

        let into = if self.into {
            if !self.strip_option {
                let ty = &self.ty;
                quote! {
                    fn transform<__IntoReactiveValueMarker>(value: impl ::leptos::prelude::IntoReactiveValue<#ty, __IntoReactiveValueMarker>) -> #ty {
                        value.into_reactive_value()
                    },
                }
            } else {
                let ty = unwrap_option(self.ty);
                quote! {
                    fn transform<__IntoReactiveValueMarker>(value: impl ::leptos::prelude::IntoReactiveValue<#ty, __IntoReactiveValueMarker>) -> Option<#ty> {
                        Some(value.into_reactive_value())
                    },
                }
            }
        } else {
            quote! {}
        };

        let setter = if !strip_option.is_empty() || !into.is_empty() {
            quote! { setter(#strip_option #into) }
        } else {
            quote! {}
        };

        let output = if !default.is_empty() || !setter.is_empty() {
            quote! { #[builder(#default #setter)] }
        } else {
            quote! {}
        };

        tokens.append_all(output);
    }
}
