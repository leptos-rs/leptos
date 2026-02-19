use proc_macro2::Ident;
use syn::{Type, TypePath};

/// The set of type names that are recognized as "children" types.
const CHILDREN_TYPE_NAMES: &[&str] = &[
    "Children",
    "ChildrenFn",
    "ChildrenFnMut",
    "ChildrenFragment",
    "ChildrenFragmentFn",
    "ChildrenFragmentMut",
    "TypedChildren",
    "TypedChildrenFn",
    "TypedChildrenMut",
];

/// Returns `true` if a prop is a children prop — detected by name
/// `"children"` or by having a known children type.
pub(crate) fn is_children_prop(name: &Ident, ty: &Type) -> bool {
    name == "children" || is_children_type(ty)
}

/// Returns `true` if `ty` is a known children type (e.g. `Children`, `TypedChildrenFn<C>`) or
/// `Option<ChildrenFn>`.
fn is_children_type(ty: &Type) -> bool {
    is_children_type_inner(ty, true)
}

fn is_children_type_inner(ty: &Type, allow_option: bool) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        if let Some(last) = path.segments.last() {
            let name = last.ident.to_string();
            if CHILDREN_TYPE_NAMES.iter().any(|&n| n == name) {
                return true;
            }
            // Unwrap one layer of Option<T>
            if allow_option && name == "Option" {
                if let syn::PathArguments::AngleBracketed(args) =
                    &last.arguments
                {
                    if let Some(syn::GenericArgument::Type(inner)) =
                        args.args.first()
                    {
                        return is_children_type_inner(inner, false);
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::Span;

    fn parse_ty(s: &str) -> Type {
        syn::parse_str(s).unwrap()
    }

    fn ident(s: &str) -> Ident {
        Ident::new(s, Span::call_site())
    }

    mod is_children_prop {
        use super::*;

        #[test]
        fn is_children_prop_by_name() {
            assert!(is_children_prop(&ident("children"), &parse_ty("String")));
        }

        #[test]
        fn is_children_prop_by_type() {
            assert!(is_children_prop(&ident("content"), &parse_ty("Children")));
            assert!(is_children_prop(
                &ident("body"),
                &parse_ty("TypedChildrenFn<C>")
            ));
        }

        #[test]
        fn is_children_prop_neither() {
            assert!(!is_children_prop(&ident("label"), &parse_ty("String")));
        }
    }

    mod is_children_type {
        use super::*;

        #[test]
        fn is_children_type_bare_children() {
            assert!(is_children_type(&parse_ty("Children")));
        }

        #[test]
        fn is_children_type_bare_children_fn() {
            assert!(is_children_type(&parse_ty("ChildrenFn")));
        }

        #[test]
        fn is_children_type_bare_children_fn_mut() {
            assert!(is_children_type(&parse_ty("ChildrenFnMut")));
        }

        #[test]
        fn is_children_type_typed() {
            assert!(is_children_type(&parse_ty("TypedChildren<C>")));
            assert!(is_children_type(&parse_ty("TypedChildrenFn<C>")));
            assert!(is_children_type(&parse_ty("TypedChildrenMut<C>")));
        }

        #[test]
        fn is_children_type_fragment_variants() {
            assert!(is_children_type(&parse_ty("ChildrenFragment")));
            assert!(is_children_type(&parse_ty("ChildrenFragmentFn")));
            assert!(is_children_type(&parse_ty("ChildrenFragmentMut")));
        }

        #[test]
        fn is_children_type_option_wrapped() {
            assert!(is_children_type(&parse_ty("Option<Children>")));
            assert!(is_children_type(&parse_ty("Option<TypedChildrenFn<C>>")));
        }

        #[test]
        fn is_children_type_not_children() {
            assert!(!is_children_type(&parse_ty("i32")));
            assert!(!is_children_type(&parse_ty("String")));
            assert!(!is_children_type(&parse_ty("Option<String>")));
            assert!(!is_children_type(&parse_ty("Vec<Children>")));
        }

        #[test]
        fn is_children_type_double_option_not_unwrapped() {
            assert!(!is_children_type(&parse_ty("Option<Option<Children>>")));
        }
    }
}
