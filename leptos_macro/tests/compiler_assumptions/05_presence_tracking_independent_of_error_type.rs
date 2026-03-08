// Assumption: Combining the two-step pre-check (test 04) with sentinel-based presence
// tracking (test 03), errors fire independently. The `{error}` from `.extract_value()` does
// NOT suppress the E0277 from `.require_props()`, because they share no values.
//
// This is the assumption behind the complete flow in `component_builder.rs` —
// `require_props()` + `check_and_wrap_*` + `check_missing` all fire independently.
//
// Expected: 4 errors firing independently:
// 1. E0277 — missing `bar` (from `require_props` where-clause)
// 2. E0277 — wrong type for `foo` (from `check_and_wrap_foo` trait bound)
// 3. E0599 — `extract_value` bounds not satisfied (from bounded inherent on `WrapFoo`)
// 4. E0599 — `check_missing` bounds not satisfied (from bounded inherent on `PresenceBuilder`)
// No downstream errors (all absorbed by `{error}`).

struct Present;
struct Absent;

struct Helper;

// -- Wrong-type check (like test 04) --
#[diagnostic::on_unimplemented(
    message = "wrong type: `{Self}` is not of type 'Fn() -> bool'"
)]
trait CheckFoo: Fn() -> bool {}
impl<T: Fn() -> bool> CheckFoo for T {}

struct WrapFoo<T>(T);
impl<T: Fn() -> bool> WrapFoo<T> {
    fn extract_value(self) -> T {
        self.0
    }
}

impl Helper {
    fn check_and_wrap_foo<T: CheckFoo>(&self, val: T) -> WrapFoo<T> {
        WrapFoo(val)
    }
}

// -- Presence tracking (like test 03) --
#[diagnostic::on_unimplemented(
    message = "missing required prop `bar`",
    label = "missing prop `bar`"
)]
trait RequiredBar {}
impl RequiredBar for Present {}

struct PresenceBuilder<S>(std::marker::PhantomData<S>);

// Unbounded impl: require_props with where-clause (borrows &self)
impl<F0, F1> PresenceBuilder<(F0, F1)> {
    fn require_props(&self)
    where
        F1: RequiredBar,
    {
    }
}

// Bounded impl: check_missing only available when all required present (takes self)
impl<F0, F1: RequiredBar> PresenceBuilder<(F0, F1)> {
    fn check_missing<B>(self, builder: B) -> B {
        builder
    }
}

fn needs_string(_: String) {}

fn main() {
    let helper = Helper;

    // foo present (wrong type) + bar absent
    let presence = PresenceBuilder::<(Present, Absent)>(std::marker::PhantomData);
    presence.require_props(); // E0277 (missing bar)

    let checked_foo = helper.check_and_wrap_foo(true).extract_value(); // E0277 + E0599 (wrong type)

    let builder = 42i32;
    let builder = presence.check_missing(builder); // E0599 (missing bar -> {error})

    needs_string(builder); // absorbed by {error}
    needs_string(checked_foo); // absorbed by {error}
}
