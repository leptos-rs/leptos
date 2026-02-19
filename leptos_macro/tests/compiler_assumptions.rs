// Trybuild tests that verify rustc compiler behavior assumptions
// relied upon by the error localization system.
//
// These tests use self-contained Rust code (no leptos macros).
// If any assumption changes in a future rustc version, the stderr
// snapshots will diverge and these tests will fail.
//
// Run: cargo +nightly test -p leptos_macro --test compiler_assumptions
// Update: TRYBUILD=overwrite cargo +nightly test -p leptos_macro --test compiler_assumptions
#[test]
fn compiler_assumptions() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compiler_assumptions/01_ufcs_no_error_type.rs");
    t.compile_fail(
        "tests/compiler_assumptions/02_method_produces_error_type.rs",
    );
    t.compile_fail(
        "tests/compiler_assumptions/\
         03_method_no_on_unimplemented_for_closures.rs",
    );
}
