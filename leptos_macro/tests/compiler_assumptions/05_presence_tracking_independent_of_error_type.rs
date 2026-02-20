// Assumption: Type-level checks are independent of {error} contamination.
//
// Combining the two-step pre-check (04) with a type-level presence check (03)
// produces errors from both independently. The {error} from the method call does
// NOT suppress the E0277 from the presence check, because they share no values.
//
// Expected errors:
// - E0277 (with our custom on_unimplemented message from the UFCS check).
// - E0599 (method cannot be called due to unsatisfied trait bounds).
// - E0277 (with our custom on_unimplemented message from the presence check).

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

#[diagnostic::on_unimplemented(message = "some required props are missing")]
trait RequireProps {}

// This do_not_recommend removes a (useless to the user) hint from the last error produced by the
// `require_props` call.
#[diagnostic::do_not_recommend]
impl<__T0, __T1> RequireProps for ((__T0,), (__T1,)) {}

fn require_props<S: RequireProps>() {}

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

    // Presence builder: slot 0 present, slot 1 missing.
    // This is independent of {error} above — E0277 (missing prop, clean)
    require_props::<((String,), ())>();

    // The key point is: The `require_props` (type-level only) call and the check/pass combo
    // (operating on `val`) SHARE NO VALUES with each other. That's why the {error} resolved for
    // the second `let val ...` doesn't contaminate.

    // Downstream usage of {error}-resolved type produces no errors.
    needs_string(val);
}
