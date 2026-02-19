// Assumption: UFCS calls do NOT produce {error} type.
//
// When `<bool as Check>::pass(val)` fails (E0277), the return type
// is resolved as `bool` from the trait definition (Self = bool),
// NOT as {error}. Downstream code still type-checks and errors.
//
// Expected: 2 errors — E0277 + E0308.
// If UFCS produced {error}, we'd see only 1 error (E0277).

#[diagnostic::on_unimplemented(
    message = "custom: `{Self}` does not implement `Check`"
)]
trait Check {
    fn pass(self) -> Self;
}

fn needs_string(_: String) {}

fn main() {
    let val: bool = true;
    let result = <bool as Check>::pass(val);
    needs_string(result);
}
