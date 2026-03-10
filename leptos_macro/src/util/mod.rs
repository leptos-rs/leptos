//! Shared utility functions for the `#[component]` and `#[slot]`
//! macros.
//!
//! These helpers are used by both `component.rs` and `slot.rs` to
//! generate per-prop type checks, required-prop checks, and phantom
//! fields — all part of the localized error reporting machinery.

pub mod children;
pub mod companion_module;
pub mod documentation;
pub mod type_analysis;
pub mod typed_builder_opts;

pub(crate) use companion_module::{
    generate_companion_internals, CompanionConfig, CompanionModuleBody,
};
use documentation::Docs;
use proc_macro2::Ident;
use syn::{Type, TypePath};

/// Trait abstracting over component and slot prop types.
///
/// Implemented by `ComponentProp` and `SlotProp` so that
/// [`generate_companion_internals`] and friends can be generic over
/// both.
pub(crate) trait PropLike {
    fn name(&self) -> &Ident;

    fn ty(&self) -> &Type;

    fn docs(&self) -> &Docs;

    /// Determines whether this prop should be considered optional.
    /// Multiple user settings can lead to the prop being optional.
    /// Do not conflate with `optional`, which just provides info
    /// whether the raw `#[prop(optional)]` flag was specified.
    fn is_optional(&self) -> bool;
    fn is_required(&self) -> bool {
        !self.is_optional()
    }

    /// Whether the `#[prop(optional)]` flag was specified.
    fn has_optional_flag(&self) -> bool;

    /// Whether the `#[prop(strip_option)]` flag was specified.
    fn has_strip_option_flag(&self) -> bool;

    /// Whether the `#[prop(into)]` flag was specified.
    fn has_into_flag(&self) -> bool;

    /// Optional user-provided expression creating a
    /// default value of this prop.
    fn default(&self) -> Option<&syn::Expr>;
}

/// Strips the raw identifier prefix (`r#`) from a prop name.
pub(crate) fn strip_raw_prefix(ident: &Ident) -> String {
    let s = ident.to_string();
    s.strip_prefix("r#").unwrap_or(&s).to_owned()
}

/// Unwraps `Option<T>` to `T`. Aborts if the type is not
/// `Option<T>`.
pub(crate) fn unwrap_option(ty: &Type) -> Type {
    const STD_OPTION_MSG: &str =
        "make sure you're not shadowing the `std::option::Option` type that \
         is automatically imported from the standard prelude";

    if let Type::Path(TypePath {
        path: syn::Path { segments, .. },
        ..
    }) = ty
    {
        if segments.len() == 1 {
            let first = segments.first().unwrap();
            if first.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(
                    syn::AngleBracketedGenericArguments { args, .. },
                ) = &first.arguments
                {
                    if args.len() == 1 {
                        if let syn::GenericArgument::Type(ty) =
                            args.first().unwrap()
                        {
                            return ty.clone();
                        }
                    }
                }
            }
        }
    }

    proc_macro_error2::abort!(
        ty,
        "`Option` must be `std::option::Option`";
        help = STD_OPTION_MSG
    );
}
