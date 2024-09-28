#[cfg(not(erase_components))]
#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/component.rs");
    t.compile_fail("tests/ui/component_absolute.rs");
    t.compile_fail("tests/ui/server.rs");
}
