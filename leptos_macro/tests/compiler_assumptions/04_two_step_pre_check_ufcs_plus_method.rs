// Assumption: Combining UFCS (01) and method call (02) produces exactly 2 errors.
//
// The UFCS check `<_ as IsI32>::check(&val)` produces E0277 with on_unimplemented.
// The method call `val.pass()` produces E0599, making the return type {error}.
// Downstream usage of the {error}-typed value is absorbed — no further errors.
//
// Expected errors:
// - E0277 (with our custom on_unimplemented message).
// - E0599 (method cannot be called due to unsatisfied trait bounds).

#[diagnostic::on_unimplemented(
    message = "wrong type: `{Self}` is not of type 'i32'"
)]
trait IsI32 {
    fn check(_val: &Self) {}
    fn pass(self) -> Self
    where
        Self: Sized,
    {
        self
    }
}

impl IsI32 for i32 {}

fn needs_string(_: String) {}

fn main() {
    let val: bool = true;

    // Pre-check step 1: Type system resolves `_` to `bool`. But `bool` doesn't implement `IsI32`,
    // so E0277 is produced, showing our custom (clean) error message.
    <_ as IsI32>::check(&val);

    // Pre-check step 2: Method resolution fails, so the return type becomes {error} and E0599 is
    // produced. `let val = ...` shadows our previous `val`, so {error} is later passed to
    // `needs_string`, not leading to any additional error.
    let val = val.pass();

    // Downstream usage of {error}-resolved type produces no errors.
    needs_string(val);
}
