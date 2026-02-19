// Assumption: {error} type is absorbed by UFCS trait calls.
//
// When a value has type {error} (produced by a failed method call),
// UFCS calls on that value do NOT produce additional errors. This
// ensures that when a wrong-type prop produces {error}, the
// `__Finish` UFCS call is silently absorbed.
//
// Expected: 1 error — E0599 from the method call only.
// The UFCS `<_ as Finish>::finish(builder)` does NOT produce
// a second error.

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

struct Builder<S>(S);
struct MyStruct;

#[diagnostic::on_unimplemented(
    message = "component cannot be created — some required props are missing"
)]
trait Finish {
    type Output;
    fn finish(self) -> Self::Output;
}

impl Finish for Builder<((bool,),)> {
    type Output = MyStruct;
    fn finish(self) -> Self::Output {
        MyStruct
    }
}

fn needs_my_struct(_: MyStruct) {}

fn main() {
    let val: bool = true;
    // Method call produces {error}
    let checked = val.pass();
    // Feed {error} into Builder
    let builder = Builder(((checked,),));
    // UFCS call — should be absorbed by {error}
    let result = <_ as Finish>::finish(builder);
    needs_my_struct(result);
}
