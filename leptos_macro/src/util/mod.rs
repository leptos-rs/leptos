//! Shared utility functions for the `#[component]` and `#[slot]`
//! macros.
//!
//! These helpers are used by both `component.rs` and `slot.rs` to
//! generate per-prop type checks, required-prop checks, and phantom
//! fields — all part of the localized error reporting machinery.

pub mod children;
pub mod companion_module;
pub mod documentation;
pub mod prop_checks;
pub mod property_documentation;
pub mod type_analysis;
pub mod typed_builder_opts;

pub(crate) use companion_module::{
    generate_companion_internals, CompanionConfig, CompanionModuleBody,
};
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
    fn is_required(&self) -> bool;
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

/// Returns true if the type is `Option<_>`.
pub(crate) fn is_option(ty: &Type) -> bool {
    if let Type::Path(TypePath {
        path: syn::Path { segments, .. },
        ..
    }) = ty
    {
        segments.len() == 1 && segments.first().unwrap().ident == "Option"
    } else {
        false
    }
}

// ── Name validation ─────────────────────────────────────────────

/// A list of names which cannot be used by Leptos users to name their own
/// components.
const FORBIDDEN_TYPE_NAMES: &[&str] = &[
    "Component",
    "Props",
    "Children",
    "View",
    "Fragment",
    "Signal",
    "Memo",
    "Effect",
    "Action",
    "Resource",
    "Callback",
    "Owner",
];

/// Checks whether the component `name` is forbidden.
///
/// `#[component]` generates a companion module with the same name as the
/// component function. Because modules live in the type namespace, they can
/// conflict with structs, traits, enums, and type aliases imported through, for
/// example, `leptos::prelude::*`, or other sources.
///
/// when a user tries to name a component the same way a leptos-provided item is
/// already named, Rust would produce an unhelpful E0659 (ambiguous name) error.
///
/// We catch the most common collisions at compile-time and emit a clear error
/// instead
pub(crate) fn check_component_name_against_prelude(name: &Ident) {
    // Skip the check when compiling the `leptos` crate itself, which
    // defines some of the items in our list (e.g. `Component`, `View`).
    if std::env::var("CARGO_PKG_NAME").as_deref() == Ok("leptos") {
        return;
    }

    let name_str = name.to_string();
    if FORBIDDEN_TYPE_NAMES.contains(&name_str.as_str()) {
        proc_macro_error2::abort!(
            name.span(),
            "component name `{}` conflicts with `leptos::prelude::{}`",
            name_str, name_str;
            help = "rename the component to avoid the conflict, e.g. `My{}` or `App{}`",
            name_str, name_str
        );
    }
}
