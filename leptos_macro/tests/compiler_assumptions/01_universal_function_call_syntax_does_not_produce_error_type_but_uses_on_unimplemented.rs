// Assumption: Universal function call syntax does NOT produce an {error} type.
//
// When `<bool as Check>::pass(val)` fails (E0277), the return type is resolved as `bool`,
// as described by the trait definition (Self = bool), and NOT as {error}.
//
// Downstream code will still be type-checked and will surface further errors.
//
// Expected errors:
// - E0277 (with our custom on_unimplemented message)
// - E0308 (from passing a non-string typed value to needs_string).

#[diagnostic::on_unimplemented(
    message = "custom: `{Self}` does not implement `Check`"
)]
trait Check {
    fn pass(self) -> Self;
}

// Implementing `Check` for some types (other than bool, used for test) does not change the
// outcome largely. We only get an additional note in our E0599 error referencing this blanket impl.
impl<T: Fn() -> bool> Check for T {
    fn pass(self) -> Self {
        self
    }
}

fn needs_string(_: String) {}

fn main() {
    let val: bool = true;

    // Trait `Check` is not implemented on `bool`, so this is our first error.
    // Explicitly specifying the return value here makes no difference in the output.
    // The key is: The compiler can resolve `_` to bool, because we used `val`. And it knows that
    // Check::pass is called and that it returns Self, which must be `bool` again. So the return
    // value does not become {error}.
    let result = <_ as Check>::pass(val);

    // `bool` is not compatible with `String`. This is also an error. AND the error IS reported.
    needs_string(result);
}
