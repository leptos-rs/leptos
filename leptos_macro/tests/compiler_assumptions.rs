/// Trybuild tests that verify rustc compiler behavior assumptions
/// relied upon by the way we implemented localized view! macro
/// errors.
///
/// These tests don't use any Leptos code.
/// The .stderr snapshots may slightly diverge due to changes
/// in the rust compiler. As long as the main assumptions
/// stated in the .rs files are not violated, this is fine.
///
/// Run tests with `cargo test -p leptos_macro --test compiler_assumptions`.
///
/// Update tests with `TRYBUILD=overwrite cargo test -p leptos_macro --test compiler_assumptions`.
///
/// CI runs tests on both the stable and nightly toolchains,
/// whose exact compiler output may differ in these tests.
/// `trybuild` does have support for multiple .stderr files
/// taken into account depending on the toolchain being used.
/// Therefore, we let the tests only run on stable.
#[rustversion::stable]
#[test]
fn compiler_assumptions() {
    let t = trybuild::TestCases::new();
    t.compile_fail(
        "tests/compiler_assumptions/\
         01_trait_bounded_generic_param_produces_e0277_with_on_unimplemented.\
         rs",
    );
    t.compile_fail(
        "tests/compiler_assumptions/\
         02_bounded_inherent_method_produces_error_type.rs",
    );
    t.compile_fail(
        "tests/compiler_assumptions/\
         03_where_clause_inherent_method_per_bound_e0277_with_on_unimplemented.\
         rs",
    );
    t.compile_fail(
        "tests/compiler_assumptions/\
         04_check_and_wrap_plus_bounded_unwrap_two_errors.rs",
    );
    t.compile_fail(
        "tests/compiler_assumptions/\
         05_presence_tracking_independent_of_error_type.rs",
    );
    t.compile_fail(
        "tests/compiler_assumptions/\
         06_closure_inference_through_generic_functions.rs",
    );
}
