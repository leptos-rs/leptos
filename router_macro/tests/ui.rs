//! Compile-fail coverage for the `path!` macro's rejection rules.
//!
//! Each `.rs` file in `tests/ui/` is expected to fail compilation with the
//! diagnostic recorded in its sibling `.stderr` file.

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
