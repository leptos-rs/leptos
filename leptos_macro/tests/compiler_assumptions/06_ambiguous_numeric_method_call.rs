// Assumption: Method calls on ambiguous numeric literals produce E0689,
// not E0599.
//
// When `42.pass()` is called and the trait has a bounded impl, rustc
// produces E0689 ("can't call method on ambiguous numeric type")
// instead of E0599. This means the `{error}` propagation path may
// differ for ambiguous integer literals compared to typed values.
//
// Expected: 1 error — E0689 only.

trait Check {
    fn pass(self) -> Self;
}

impl<T: Fn() -> bool> Check for T {
    fn pass(self) -> Self {
        self
    }
}

fn needs_string(_: String) {}

fn main() {
    let result = 42.pass();
    needs_string(result);
}
