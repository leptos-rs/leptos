use leptos::{create_runtime, create_rw_signal};
use leptos_macro::Lens;

#[derive(Lens, Default)]
struct FieldStruct {
    age: u32,
    name: String,
}

#[derive(Lens, Default)]
struct TupleStruct(u32, String);

#[test]
fn green() {
    let _ = create_runtime();

    let field_signal = create_rw_signal(FieldStruct::default());
    let _ = FieldStruct::age(field_signal);
    let _ = FieldStruct::name(field_signal);

    let tuple_signal = create_rw_signal(TupleStruct::default());
    let _ = TupleStruct::_0(tuple_signal);
    let _ = TupleStruct::_1(tuple_signal);
}

#[test]
fn red() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/lens/structures.rs")
}
