use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{
    visit::Visit, GenericParam, Type, TypeParamBound, TypePath, WherePredicate,
};

/// Walks a syntax tree looking for path segments matching any of the
/// given target identifiers.
///
/// Only `visit_path_segment` is overridden because generic type
/// params always appear as path segments in type ASTs (e.g. `F`,
/// `Vec<F>`, `Option<F>`). This is sufficient for detecting whether
/// a generic param is referenced.
struct IdentFinder<'a> {
    targets: &'a [&'a Ident],
    found: bool,
}

impl<'ast> Visit<'ast> for IdentFinder<'_> {
    fn visit_path_segment(&mut self, segment: &'ast syn::PathSegment) {
        if self.targets.iter().any(|t| segment.ident == **t) {
            self.found = true;
        }
        syn::visit::visit_path_segment(self, segment);
    }
}

/// Checks whether a type AST contains a reference to the given
/// identifier (used to detect which generic params appear in field
/// types).
pub(crate) fn type_contains_ident(ty: &Type, ident: &Ident) -> bool {
    let mut finder = IdentFinder {
        targets: &[ident],
        found: false,
    };
    finder.visit_type(ty);
    finder.found
}

/// Returns true if the type is exactly the given type parameter
/// with no wrapping (e.g. `F`, not `Vec<F>`).
pub(crate) fn is_plain_type_param(ty: &Type, ident: &Ident) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        path.is_ident(ident)
    } else {
        false
    }
}

/// Returns true if the given type param appears inside a wrapping
/// type in any field (e.g. `ServerAction<ServFn>`) rather than as a
/// plain type param (e.g. `fun: F`).
pub(crate) fn param_appears_wrapped_in_fields(
    param_ident: &Ident,
    field_types: &[&Type],
) -> bool {
    field_types.iter().any(|ty| {
        type_contains_ident(ty, param_ident)
            && !is_plain_type_param(ty, param_ident)
    })
}

/// Creates a copy of the generics keeping only the bounds that are
/// structurally required by field types.
///
/// A generic param needs structural bounds when it appears wrapped
/// inside another type in a field (e.g. `ServerAction<ServFn>` needs
/// `ServFn: ServerFn`). Plain type params (e.g. `fun: F`) do not
/// need their bounds on the struct definition.
pub(crate) fn strip_non_structural_bounds(
    generics: &syn::Generics,
    field_types: &[&Type],
) -> syn::Generics {
    let mut g = generics.clone();

    // Strip inline bounds for params that don't need structural
    // bounds.
    for param in &mut g.params {
        if let GenericParam::Type(tp) = param {
            if !param_appears_wrapped_in_fields(&tp.ident, field_types) {
                tp.bounds.clear();
                tp.colon_token = None;
            }
        }
    }

    // Filter where-clause predicates.
    if let Some(where_clause) = &mut g.where_clause {
        let kept: syn::punctuated::Punctuated<_, _> = where_clause
            .predicates
            .iter()
            .filter(|pred| match pred {
                WherePredicate::Type(pt) => {
                    if let Type::Path(TypePath {
                        path, qself: None, ..
                    }) = &pt.bounded_ty
                    {
                        if let Some(ident) = path.get_ident() {
                            return param_appears_wrapped_in_fields(
                                ident,
                                field_types,
                            );
                        }
                    }
                    // Complex predicates (associated types, etc.)
                    // are kept to be safe.
                    true
                }
                // Keep lifetime bounds and any other predicates.
                _ => true,
            })
            .cloned()
            .collect();

        if kept.is_empty() {
            g.where_clause = None;
        } else {
            where_clause.predicates = kept;
        }
    }

    g
}

/// Collects where-clause predicates (both inline bounds and
/// where-clause entries) for a specific generic type param.
pub(crate) fn collect_predicates_for_param(
    generics: &syn::Generics,
    param_ident: &Ident,
) -> Vec<WherePredicate> {
    let mut preds = vec![];

    // Inline bounds (e.g., `F: Clone` in `<F: Clone>`)
    for param in &generics.params {
        if let GenericParam::Type(tp) = param {
            if tp.ident == *param_ident && !tp.bounds.is_empty() {
                let bounds = &tp.bounds;
                preds.push(syn::parse_quote! {
                    #param_ident: #bounds
                });
            }
        }
    }

    // Where-clause predicates
    if let Some(wc) = &generics.where_clause {
        for pred in &wc.predicates {
            if let WherePredicate::Type(pt) = pred {
                if let Type::Path(TypePath { path, .. }) = &pt.bounded_ty {
                    if path.get_ident() == Some(param_ident) {
                        preds.push(pred.clone());
                    }
                }
            }
        }
    }

    preds
}

/// Collects ALL predicates from a `Generics` — both inline bounds
/// (e.g. `T: Clone` in `<T: Clone>`) and where-clause predicates —
/// into a flat list of `TokenStream` fragments suitable for use in a
/// `where` clause.
pub(crate) fn collect_all_predicates(
    generics: &syn::Generics,
) -> Vec<TokenStream> {
    let mut preds = Vec::new();
    for param in &generics.params {
        if let GenericParam::Type(tp) = param {
            if !tp.bounds.is_empty() {
                let ident = &tp.ident;
                let bounds = &tp.bounds;
                preds.push(quote! { #ident: #bounds });
            }
        }
    }
    if let Some(wc) = &generics.where_clause {
        preds.extend(wc.predicates.iter().map(|p| quote! { #p }));
    }
    preds
}

/// Collects generic type params that are NOT used in any field type.
/// These need `PhantomData` so the struct compiles without their
/// where-clause bounds.
pub(crate) fn collect_phantom_type_params<'a>(
    generics: &'a syn::Generics,
    field_types: &[&Type],
) -> Vec<&'a Ident> {
    generics
        .params
        .iter()
        .filter_map(|param| {
            if let GenericParam::Type(tp) = param {
                let ident = &tp.ident;
                let used_in_field =
                    field_types.iter().any(|ty| type_contains_ident(ty, ident));
                if !used_in_field {
                    Some(ident)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

/// Generates the `PhantomData` field for a props struct if any
/// generic type params are unused in field types.
pub(crate) fn generate_phantom_field(
    phantom_type_params: &[&Ident],
    add_serde_skip: bool,
) -> TokenStream {
    if phantom_type_params.is_empty() {
        quote! {}
    } else {
        let serde_skip = if add_serde_skip {
            quote! { #[serde(skip)] }
        } else {
            quote! {}
        };
        quote! {
            #[builder(setter(skip), default)]
            #[doc(hidden)]
            #serde_skip
            _phantom: ::core::marker::PhantomData<(#(#phantom_type_params),*)>,
        }
    }
}

/// Checks if any of the bounds in the given predicates reference
/// generic params other than `self_ident`.
pub(crate) fn bounds_reference_other_params(
    predicates: &[WherePredicate],
    self_ident: &Ident,
    full_generics: &syn::Generics,
) -> bool {
    let other_idents: Vec<&Ident> = full_generics
        .params
        .iter()
        .filter_map(|p| {
            if let GenericParam::Type(tp) = p {
                if tp.ident != *self_ident {
                    return Some(&tp.ident);
                }
            }
            None
        })
        .collect();

    if other_idents.is_empty() {
        return false;
    }

    let mut finder = IdentFinder {
        targets: &other_idents,
        found: false,
    };

    for pred in predicates {
        if let WherePredicate::Type(pt) = pred {
            for bound in &pt.bounds {
                finder.visit_type_param_bound(bound);
            }
        }
    }

    finder.found
}

fn iter_bounds(
    predicates: &[WherePredicate],
) -> impl Iterator<Item = &TypeParamBound> {
    predicates
        .iter()
        .filter_map(|p| {
            if let WherePredicate::Type(pt) = p {
                Some(pt.bounds.iter())
            } else {
                None
            }
        })
        .flatten()
}

/// Returns true if any of the predicates contain an `Fn`, `FnMut`,
/// or `FnOnce` bound (including HRTB forms like `for<'a> Fn(...)`).
pub(crate) fn predicates_contain_fn_bound(
    predicates: &[WherePredicate],
) -> bool {
    iter_bounds(predicates).any(|bound| {
        if let TypeParamBound::Trait(tb) = bound {
            tb.path.segments.last().map_or(false, |seg| {
                matches!(
                    seg.ident.to_string().as_str(),
                    "Fn" | "FnMut" | "FnOnce"
                )
            })
        } else {
            false
        }
    })
}

/// Merges all type-param bounds from a list of where predicates
/// into a single `A + B + C` token stream.
pub(crate) fn merge_predicate_bounds(
    predicates: &[WherePredicate],
) -> TokenStream {
    let bounds: Vec<&TypeParamBound> = iter_bounds(predicates).collect();
    if bounds.is_empty() {
        quote! {}
    } else {
        quote! { #(#bounds)+* }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::Span;
    use quote::ToTokens;
    use syn::{parse_quote, ItemFn};

    fn parse_ty(s: &str) -> Type {
        syn::parse_str(s).unwrap()
    }

    fn ident(s: &str) -> Ident {
        Ident::new(s, Span::call_site())
    }

    /// Parse generics (including where clause) by wrapping in a
    /// function signature, since `syn::Generics` alone doesn't
    /// parse where clauses.
    fn parse_generics(s: &str) -> syn::Generics {
        let (angle_part, where_part) = if let Some(idx) = s.find("where") {
            (&s[..idx], &s[idx..])
        } else {
            (s, "")
        };
        let fn_str = format!("fn f{angle_part}() {where_part} {{}}");
        let item_fn: ItemFn = syn::parse_str(&fn_str).unwrap();
        item_fn.sig.generics
    }

    // --- type_contains_ident ---

    #[test]
    fn type_contains_ident_bare() {
        assert!(type_contains_ident(&parse_ty("F"), &ident("F")));
    }

    #[test]
    fn type_contains_ident_different() {
        assert!(!type_contains_ident(&parse_ty("F"), &ident("G")));
    }

    #[test]
    fn type_contains_ident_in_vec() {
        assert!(type_contains_ident(&parse_ty("Vec<F>"), &ident("F")));
    }

    #[test]
    fn type_contains_ident_nested() {
        assert!(type_contains_ident(
            &parse_ty("Option<Vec<F>>"),
            &ident("F")
        ));
    }

    #[test]
    fn type_contains_ident_absent() {
        assert!(!type_contains_ident(&parse_ty("Vec<i32>"), &ident("F")));
    }

    #[test]
    fn type_contains_ident_in_hashmap() {
        assert!(type_contains_ident(
            &parse_ty("std::collections::HashMap<String, F>"),
            &ident("F")
        ));
    }

    // --- is_plain_type_param ---

    #[test]
    fn plain_type_param_exact() {
        assert!(is_plain_type_param(&parse_ty("F"), &ident("F")));
    }

    #[test]
    fn plain_type_param_wrapped() {
        assert!(!is_plain_type_param(&parse_ty("Vec<F>"), &ident("F")));
    }

    #[test]
    fn plain_type_param_different_name() {
        assert!(!is_plain_type_param(&parse_ty("G"), &ident("F")));
    }

    #[test]
    fn plain_type_param_option() {
        assert!(!is_plain_type_param(&parse_ty("Option<F>"), &ident("F")));
    }

    // --- collect_predicates_for_param ---

    #[test]
    fn collect_inline_bound() {
        let generics: syn::Generics = parse_quote! { <F: Clone> };
        let preds = collect_predicates_for_param(&generics, &ident("F"));
        assert_eq!(preds.len(), 1);
    }

    #[test]
    fn collect_where_clause_bound() {
        let generics = parse_generics("<F> where F: Fn() -> bool");
        let preds = collect_predicates_for_param(&generics, &ident("F"));
        assert_eq!(preds.len(), 1);
    }

    #[test]
    fn collect_inline_and_where() {
        let generics = parse_generics("<F: Clone> where F: Send");
        let preds = collect_predicates_for_param(&generics, &ident("F"));
        assert_eq!(preds.len(), 2);
    }

    #[test]
    fn collect_different_param() {
        let generics = parse_generics("<F: Clone> where G: Send");
        let preds = collect_predicates_for_param(&generics, &ident("G"));
        assert_eq!(preds.len(), 1);
    }

    #[test]
    fn collect_no_bounds() {
        let generics: syn::Generics = parse_quote! { <F> };
        let preds = collect_predicates_for_param(&generics, &ident("F"));
        assert_eq!(preds.len(), 0);
    }

    // --- collect_all_predicates ---

    #[test]
    fn all_preds_empty_generics() {
        let generics: syn::Generics = parse_quote! { <F> };
        let preds = collect_all_predicates(&generics);
        assert!(preds.is_empty());
    }

    #[test]
    fn all_preds_inline_only() {
        let generics: syn::Generics = parse_quote! { <F: Clone, G: Send> };
        let preds = collect_all_predicates(&generics);
        assert_eq!(preds.len(), 2);
    }

    #[test]
    fn all_preds_where_clause_only() {
        let generics = parse_generics("<F> where F: Clone, F: Send");
        let preds = collect_all_predicates(&generics);
        assert_eq!(preds.len(), 2);
    }

    #[test]
    fn all_preds_inline_and_where() {
        let generics =
            parse_generics("<F: Clone, G: Send> where F: Fn() -> bool");
        let preds = collect_all_predicates(&generics);
        assert_eq!(preds.len(), 3);
    }

    #[test]
    fn all_preds_skips_lifetimes() {
        let generics: syn::Generics = parse_quote! { <'a, F: Clone> };
        let preds = collect_all_predicates(&generics);
        assert_eq!(preds.len(), 1);
    }

    // --- param_appears_wrapped_in_fields ---

    #[test]
    fn wrapped_plain_param_no() {
        let ty = parse_ty("F");
        assert!(!param_appears_wrapped_in_fields(&ident("F"), &[&ty]));
    }

    #[test]
    fn wrapped_in_vec_yes() {
        let ty = parse_ty("Vec<F>");
        assert!(param_appears_wrapped_in_fields(&ident("F"), &[&ty]));
    }

    #[test]
    fn wrapped_concrete_no() {
        let ty = parse_ty("i32");
        assert!(!param_appears_wrapped_in_fields(&ident("F"), &[&ty]));
    }

    #[test]
    fn wrapped_in_server_action_yes() {
        let ty = parse_ty("ServerAction<F>");
        assert!(param_appears_wrapped_in_fields(&ident("F"), &[&ty]));
    }

    // --- strip_non_structural_bounds ---

    #[test]
    fn strip_bare_generic() {
        let generics = parse_generics("<F: Fn()> where F: Fn()");
        let ty = parse_ty("F");
        let stripped = strip_non_structural_bounds(&generics, &[&ty]);
        if let GenericParam::Type(tp) = &stripped.params[0] {
            assert!(tp.bounds.is_empty());
        } else {
            panic!("expected type param");
        }
        assert!(stripped.where_clause.is_none());
    }

    #[test]
    fn strip_keeps_structural() {
        let generics = parse_generics("<F: Clone> where F: Clone");
        let ty = parse_ty("Vec<F>");
        let stripped = strip_non_structural_bounds(&generics, &[&ty]);
        if let GenericParam::Type(tp) = &stripped.params[0] {
            assert!(!tp.bounds.is_empty());
        } else {
            panic!("expected type param");
        }
        assert!(stripped.where_clause.is_some());
    }

    #[test]
    fn strip_mixed() {
        let generics = parse_generics("<F: Fn(), S: Clone> where S: Clone");
        let ty_f = parse_ty("F");
        let ty_s = parse_ty("ServerAction<S>");
        let stripped = strip_non_structural_bounds(&generics, &[&ty_f, &ty_s]);
        if let GenericParam::Type(tp) = &stripped.params[0] {
            assert!(tp.bounds.is_empty(), "F bounds should be stripped");
        }
        if let GenericParam::Type(tp) = &stripped.params[1] {
            assert!(!tp.bounds.is_empty(), "S bounds should be kept");
        }
        let wc = stripped.where_clause.as_ref().unwrap();
        assert_eq!(wc.predicates.len(), 1);
        let pred_str = wc.predicates[0].to_token_stream().to_string();
        assert!(
            pred_str.contains("S"),
            "kept predicate should be for S, got: {pred_str}"
        );
    }
}
