#![cfg(feature = "browser")]

#[test]
fn aliased_results() {
    let t = trybuild::TestCases::new();
    t.pass("tests/valid/*.rs");
    t.compile_fail("tests/invalid/*.rs")
}
