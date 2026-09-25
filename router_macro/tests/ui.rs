//! Compile-fail coverage for the router macros' rejection rules.
//!
//! Each `.rs` file in `tests/ui/` is expected to fail compilation with the
//! diagnostic recorded in its sibling `.stderr` file.

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    // `lazy_route_preserves_input` references an associated item from the
    // rejected impl. Keeping the input as dummy output prevents a second
    // "associated item not found" diagnostic.
    t.compile_fail("tests/ui/*.rs");
}
