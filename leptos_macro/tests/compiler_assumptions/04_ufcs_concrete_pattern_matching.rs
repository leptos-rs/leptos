// Assumption: UFCS with concrete tuple-pattern matching trait impl
// produces a clean E0277 with `on_unimplemented` when the pattern
// doesn't match, and the associated type is resolved via
// bidirectional type inference when there IS a usage context.
//
// This is the basis for `__Finish`: we generate an impl with
// concrete `(T,)` patterns for required type-state slots. When a
// required prop is missing, the type-state slot is `()` (not `(T,)`),
// the impl doesn't match, and E0277 fires with a clean message.
//
// Expected: 1 error — E0277 with `on_unimplemented`.
// The downstream `needs_my_struct(result)` compiles because the
// associated type `Output` is resolved from the function argument.

struct Builder<S>(S);

struct MyStruct;

#[diagnostic::on_unimplemented(
    message = "component cannot be created — some required props are missing",
    note = "see the errors above for details on which props are missing"
)]
trait Finish {
    type Output;
    fn finish(self) -> Self::Output;
}

// Impl only matches when both slots are `(T,)`.
impl<__F0, __F1> Finish for Builder<((__F0,), (__F1,))> {
    type Output = MyStruct;
    fn finish(self) -> Self::Output {
        MyStruct
    }
}

fn needs_my_struct(_: MyStruct) {}

fn main() {
    // Slot 0 is () (missing), slot 1 is (String,) (present).
    let builder: Builder<((), (String,))> = Builder(((), (String::new(),)));
    let result = <_ as Finish>::finish(builder);
    needs_my_struct(result);
}
