// Assumption: A bounded inherent method (`impl<T: Bound> Wrap<T> { fn extract_value() }`)
// produces E0599 when bounds fail, and the return type becomes `{error}` — downstream
// errors are absorbed.
//
// Also verifies that `on_unimplemented` does NOT appear on E0599 (confirming why we
// need E0277 from test 01's pattern as the clean error).
//
// This is the assumption behind `Wrap_foo<__T>::extract_value()`. When bounds fail, E0599
// fires and `{error}` propagates, suppressing all downstream errors.
//
// Expected: Only E0599 (method exists but trait bounds not satisfied). No downstream
// E0308. Custom `on_unimplemented` message should NOT appear.

#[diagnostic::on_unimplemented(message = "custom: `{Self}` does not implement `Check`")]
trait Check {}
impl Check for i32 {}

struct Wrap<T>(T);
impl<T: Check> Wrap<T> {
    fn extract_value(self) -> T {
        self.0
    }
}

fn needs_string(_: String) {}

fn main() {
    let wrap = Wrap(true);
    let result = wrap.extract_value(); // E0599 -> {error}
    needs_string(result); // absorbed — no error
}
