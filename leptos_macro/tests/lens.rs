use leptos::{create_runtime, create_rw_signal};
use leptos_macro::{poke, Lens};

#[derive(Lens, Default)]
struct FieldStruct {
    age: u32,
    name: String,
}

#[derive(Lens, Default)]
struct TupleStruct(u32, String);

#[derive(Lens, Default)]
pub struct OuterState {
    count: i32,
    inner: InnerState,
}

#[derive(Lens, Clone, PartialEq, Default)]
pub struct InnerState {
    inner_count: i32,
    inner_name: String,
}

#[test]
fn green() {
    let _ = create_runtime();

    let field_signal = create_rw_signal(FieldStruct::default());
    let _ = FieldStruct::age(field_signal);
    let _ = FieldStruct::name(field_signal);

    let tuple_signal = create_rw_signal(TupleStruct::default());
    let _ = TupleStruct::_0(tuple_signal);
    let _ = TupleStruct::_1(tuple_signal);

    let outer_signal = create_rw_signal(OuterState::default());
    let _ = OuterState::count(outer_signal);
    let _ = OuterState::inner(outer_signal);

    let (_, _) = poke!(outer_signal.inner.inner_count);
    let (_, _) = poke!(outer_signal.inner.inner_name);
}

#[test]
fn red() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/lens/structures.rs")
}
