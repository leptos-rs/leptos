// Assumption: The full two-step flow produces exactly 2 errors and absorbs downstream:
// (1) helper method with trait-bounded generic -> E0277 with `on_unimplemented`,
// (2) bounded inherent `.extract_value()` -> E0599 -> `{error}`, downstream absorbed.
//
// This is the assumption behind `__helper.check_and_wrap_foo(val).extract_value()` — the
// complete IndependentBounds pre-check.
//
// Expected: Exactly 2 errors — E0277 (custom message) + E0599 (method exists but bounds
// not satisfied). No downstream E0308.

struct Helper;

#[diagnostic::on_unimplemented(message = "`{Self}` is not a valid type for prop `foo`")]
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

fn needs_string(_: String) {}

fn main() {
    let helper = Helper;
    let result = helper.check_and_wrap_foo(true).extract_value(); // E0277 + E0599
    needs_string(result); // absorbed
}
