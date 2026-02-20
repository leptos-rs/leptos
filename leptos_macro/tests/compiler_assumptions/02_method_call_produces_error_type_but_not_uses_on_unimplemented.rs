// Assumption: Method calls DO produce an {error} type.
//
// When `val.pass()` fails (E0599), the return type IS {error}, which unifies with any type.
// Downstream errors, e.g. passing the wrong value to needs_string, are suppressed.
//
// Expected errors:
// - E0599 (because we did not state the trait in any way when calling `pass`.).

#[diagnostic::on_unimplemented(
    message = "custom: `{Self}` does not implement `Check`"
)]
trait Check {
    fn pass(self) -> Self;
}

// Implementing `Check` for some types (other than bool, used for test) does not change the
// outcome largely. We only get an additional note in our E0277 error referencing this blanket impl.
impl<T: Fn() -> bool> Check for T {
    fn pass(self) -> Self {
        self
    }
}

fn needs_string(_: String) {}

fn main() {
    // The outcome of this test does not change with more complex typed values here,
    // e.g. `|| "foo"`.
    let val: bool = true;

    // We call an arbitrary `pass` fn here, not implemented on `bool`, so this is our first error.
    // This `pass` is in now way associated with our `Check` trait, so no on_unimplemented message
    // will be surfaced.
    //
    // This unresolvable method call makes the return type unify with "anything",
    // suppressing all downstream errors!
    //
    // It doesn't matter WHY this method does not exist. It could also be a bounded inherent
    // method, for which its impl bounds simply weren't satisfied.
    let result = val.pass();

    // `result` is resolved to {error},
    needs_string(result);
}
