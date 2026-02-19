// Assumption: E0599 does NOT show #[diagnostic::on_unimplemented]
// for closure types.
//
// When a method call on a closure fails with E0599, the custom
// message from on_unimplemented is NOT displayed. This is why UFCS
// (E0277) is needed as the first step — it always shows clean
// messages.
//
// Expected: E0599 WITHOUT "CUSTOM_MESSAGE_MARKER" in the output.

#[diagnostic::on_unimplemented(message = "CUSTOM_MESSAGE_MARKER")]
trait Check {
    fn pass(self) -> Self;
}

impl<T: Fn(i32) -> i32> Check for T {
    fn pass(self) -> Self {
        self
    }
}

fn main() {
    let val = || true;
    let _ = val.pass();
}
