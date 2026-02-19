// Assumption: Method calls DO produce {error} type.
//
// When `val.pass()` fails (E0599), the return type is {error},
// which unifies with any type. Downstream errors are suppressed.
//
// Expected: 1 error — E0599 only.
// If method calls didn't produce {error}, we'd also see E0308.

#[diagnostic::on_unimplemented(
    message = "custom: `{Self}` does not implement `Check`"
)]
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
    let val: bool = true;
    let result = val.pass();
    needs_string(result);
}
