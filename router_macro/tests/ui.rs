#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    // The fixture references an associated item from the rejected impl. Keeping
    // the input as dummy output prevents a second "associated item not found"
    // diagnostic.
    t.compile_fail("tests/ui/lazy_route_preserves_input.rs");
}
