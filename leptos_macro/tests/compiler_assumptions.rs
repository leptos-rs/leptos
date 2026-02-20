// Trybuild tests that verify rustc compiler behavior assumptions relied upon by the way we
// implemented localized view! macro errors.
//
// These tests don't use any Leptos code.
// The .stderr snapshots may slightly diverge due to changes in the rust compiler.
// As long as the main assumptions stated in the .rs files are not violated, this is fine.
//
// Note: run with `cargo +nightly test -p leptos_macro --test compiler_assumptions`.
// Note: update with `TRYBUILD=overwrite cargo +nightly test -p leptos_macro --test compiler_assumptions`.
#[test]
fn compiler_assumptions() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compiler_assumptions/01_universal_function_call_syntax_does_not_produce_error_type_but_uses_on_unimplemented.rs");
    t.compile_fail(
        "tests/compiler_assumptions/\
         02_method_call_produces_error_type_but_not_uses_on_unimplemented.rs",
    );
    t.compile_fail(
        "tests/compiler_assumptions/\
         03_marker_trait_on_tuples_uses_on_unimplemented.rs",
    );
    t.compile_fail(
        "tests/compiler_assumptions/04_two_step_pre_check_ufcs_plus_method.rs",
    );
    t.compile_fail(
        "tests/compiler_assumptions/\
         05_presence_tracking_independent_of_error_type.rs",
    );
}
