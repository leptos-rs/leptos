#[cfg(not(feature = "__internal_erase_components"))]
#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    #[cfg(all(feature = "nightly", rustc_nightly))]
    t.compile_fail("tests/ui/component.rs");
    #[cfg(all(feature = "nightly", rustc_nightly))]
    t.compile_fail("tests/ui/component_absolute.rs");
    t.compile_fail("tests/ui/server.rs");
}
