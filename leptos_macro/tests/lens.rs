#[test]
fn lens() {
    let t = trybuild::TestCases::new();
    t.pass("tests/lens/field_copy.rs");
}
