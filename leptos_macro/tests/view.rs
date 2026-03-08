// Note: run with `cargo +nightly test -p leptos_macro --test view`.
// Note: update with `TRYBUILD=overwrite cargo +nightly test -p leptos_macro
// --test view`.
#[test]
fn test_view_macro() {
    let t = trybuild::TestCases::new();

    // Basics.
    t.pass("tests/view/01_empty_view.rs");

    // Concrete props (simple component, no generics).
    t.pass("tests/view/02_concrete_prop.rs");
    t.compile_fail("tests/view/03_concrete_prop_wrong_type.rs");
    t.compile_fail("tests/view/04_concrete_prop_wrong_type_short_form.rs");
    t.compile_fail("tests/view/05_concrete_prop_missing.rs");
    t.compile_fail("tests/view/06_concrete_props_multiple_one_wrong_type.rs");

    // Generic props (component with generic type params + children).
    t.pass("tests/view/07_generic_component_all_props_correct.rs");
    t.compile_fail("tests/view/08_generic_prop_missing.rs");
    t.compile_fail("tests/view/09_generic_prop_wrong_type.rs");
    t.compile_fail("tests/view/10_generic_prop_wrong_type_short_form.rs");
    t.compile_fail(
        "tests/view/11_concrete_prop_wrong_type_in_generic_component.rs",
    );
    t.pass("tests/view/12_multiple_generic_params.rs");
    t.compile_fail("tests/view/13_multiple_generic_params_first_wrong_type.rs");
    t.compile_fail(
        "tests/view/14_multiple_generic_params_second_wrong_type.rs",
    );

    // Children.
    t.compile_fail("tests/view/15_children_missing.rs");
    t.compile_fail("tests/view/16_children_fn_once_instead_of_fn.rs");

    // Prop attributes.
    t.pass("tests/view/17_optional_prop.rs");
    t.compile_fail("tests/view/18_optional_prop_wrong_type.rs");
    t.pass("tests/view/19_default_prop.rs");
    t.compile_fail("tests/view/20_default_prop_wrong_type.rs");
    t.pass("tests/view/21_optional_no_strip_prop.rs");
    t.compile_fail("tests/view/22_optional_no_strip_prop_wrong_type.rs");
    t.pass("tests/view/23_strip_option_prop.rs");
    t.compile_fail("tests/view/24_strip_option_prop_wrong_type.rs");
    t.pass("tests/view/25_into_prop.rs");
    t.compile_fail("tests/view/26_into_prop_wrong_type.rs");
    t.pass("tests/view/27_into_optional_prop.rs");
    t.compile_fail("tests/view/28_into_optional_prop_wrong_type.rs");
    t.pass("tests/view/29_into_strip_option_prop.rs");
    t.compile_fail("tests/view/30_into_strip_option_prop_wrong_type.rs");

    // Builder syntax.
    t.pass("tests/view/31_builder_syntax_props.rs");
    t.pass("tests/view/32_builder_syntax_generic_component.rs");
    t.pass("tests/view/33_builder_syntax_direct_struct.rs");

    // Let syntax.
    t.pass("tests/view/34_let_syntax_simple.rs");
    t.pass("tests/view/35_let_syntax_optional_generic_passthrough.rs");
    t.pass("tests/view/36_let_syntax_for_destructuring.rs");

    // Slots.
    t.pass("tests/view/37_slot.rs");
    t.compile_fail("tests/view/38_slot_generic_prop_wrong_type.rs");

    // Raw identifier.
    t.pass("tests/view/39_raw_identifier.rs");

    // Rename imported components.
    t.pass("tests/view/40_renamed_import_of_no_props_comp.rs");
    t.pass("tests/view/41_renamed_import_of_comp_with_props.rs");

    // Combined error scenarios.
    t.compile_fail("tests/view/42_multiple_missing_required_props.rs");
    t.compile_fail("tests/view/43_multiple_wrong_type_props.rs");
    t.compile_fail("tests/view/44_wrong_type_and_missing_prop.rs");
    t.pass("tests/view/45_only_optional_props.rs");
    t.compile_fail("tests/view/46_slot_missing_required_prop.rs");

    // Robustness tests.
    t.pass("tests/view/47_lifetime_parameterized_component.rs");
    t.pass("tests/view/48_multiple_components_same_prop_names.rs");

    // Children error messages.
    t.compile_fail("tests/view/49_children_wrong_return_type.rs");
    t.compile_fail("tests/view/50_children_wrong_type_and_missing_prop.rs");

    // Duplicate props.
    t.compile_fail("tests/view/51_duplicate_prop.rs");
    t.compile_fail("tests/view/52_duplicate_optional_prop.rs");
    t.compile_fail("tests/view/53_slot_duplicate_prop.rs");
    t.compile_fail("tests/view/54_duplicate_generic_prop.rs");
    t.compile_fail("tests/view/58_duplicate_into_prop.rs");

    // Non-Fn generic bounds.
    t.compile_fail("tests/view/55_generic_non_fn_bound_wrong_type.rs");
    t.compile_fail("tests/view/56_generic_clone_and_fn_wrong_type.rs");

    // Children-only component.
    t.compile_fail("tests/view/57_children_only_missing.rs");

    // Error isolation between components.
    t.compile_fail("tests/view/59_two_components_one_error.rs");

    // User-defined trait bounds.
    t.pass("tests/view/60_user_trait_bound_correct.rs");
    t.compile_fail("tests/view/61_user_trait_bound_wrong_type.rs");

    // Renamed slot imports.
    t.pass("tests/view/62_renamed_slot_import.rs");

    // Closure parameter type inference.
    t.pass("tests/view/63_generic_closure_with_params.rs");
}
