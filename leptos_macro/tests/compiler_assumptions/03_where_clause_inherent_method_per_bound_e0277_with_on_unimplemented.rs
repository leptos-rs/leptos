// Assumption: An inherent method with per-prop marker trait bounds in a where-clause
// fires a separate E0277 with custom `on_unimplemented` for each unsatisfied bound.
// Uses `Present`/`Absent` sentinel types (not tuples).
//
// This is the assumption behind `PresenceBuilder::require_props(&self) where
// F0: required_foo, F1: required_bar`. Each missing required prop fires an
// independent E0277.
//
// Expected: Two separate E0277 errors, each with their custom `on_unimplemented` message.

struct Present;
struct Absent;

#[diagnostic::on_unimplemented(
    message = "missing required prop `foo`",
    label = "missing prop `foo`",
    note = "required props: [`foo`, `bar`]"
)]
trait RequiredFoo {}
impl RequiredFoo for Present {}

#[diagnostic::on_unimplemented(
    message = "missing required prop `bar`",
    label = "missing prop `bar`",
    note = "required props: [`foo`, `bar`]"
)]
trait RequiredBar {}
impl RequiredBar for Present {}

struct PresenceBuilder<S>(std::marker::PhantomData<S>);

impl<F0, F1> PresenceBuilder<(F0, F1)> {
    fn require_props(&self)
    where
        F0: RequiredFoo,
        F1: RequiredBar,
    {
    }
}

fn main() {
    let p = PresenceBuilder::<(Absent, Absent)>(std::marker::PhantomData);
    p.require_props(); // E0277 for foo + E0277 for bar
}
