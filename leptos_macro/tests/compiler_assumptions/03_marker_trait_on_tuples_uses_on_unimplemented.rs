// Assumption: Marker trait on tuples with `on_unimplemented` produces clean E0277.
//
// When a marker trait is implemented only for `((__T0,), (__T1,))` but called with
// `((), (String,))` (one slot empty), the impl doesn't match and E0277 fires with the
// custom `on_unimplemented` message.
//
// Expected errors:
// - E0277 (with our custom on_unimplemented message).

#[diagnostic::on_unimplemented(message = "some required props are missing")]
trait RequireProps {}

// Only satisfied when all slots are `(T,)` (and not `()`).
impl<__T0, __T1> RequireProps for ((__T0,), (__T1,)) {}

fn require_props<S: RequireProps>() {}

fn main() {
    // Slot 0 is `()` (missing), slot 1 is `(String,)` (present).
    // This should trigger E0277 with on_unimplemented.
    require_props::<((), (String,))>();
}
