// Assumption: A generic method with a trait-bounded parameter produces E0277 with
// `on_unimplemented` when the bound fails, but does NOT produce `{error}` type —
// downstream errors still fire.
//
// This is the assumption behind `Helper::check_and_wrap_*` methods. The
// `__T: Check_foo` bound fires E0277. The return type `Wrap_foo<__T>` is still
// resolved (not `{error}`), which is critical — `{error}` propagation happens in
// step 2 (`.extract_value()`), not here (see test 02).
//
// Expected: E0277 with custom message + E0308 downstream (proving no `{error}`).

struct Helper;

#[diagnostic::on_unimplemented(message = "custom: `{Self}` does not implement `Check`")]
trait Check {}
impl Check for i32 {}

struct Wrap<T>(T);
impl<T> Wrap<T> {
    fn extract_value(self) -> T {
        self.0
    }
}

impl Helper {
    fn check_and_wrap<T: Check>(&self, val: T) -> Wrap<T> {
        Wrap(val)
    }
}

fn needs_string(_: String) {}

fn main() {
    let helper = Helper;
    let result = helper.check_and_wrap(true).extract_value(); // E0277 (custom message)
    needs_string(result); // E0308 (NOT absorbed — proves no `{error}`)
}
