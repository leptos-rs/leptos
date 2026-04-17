// Assumption: Closure parameter type inference propagates through the expected
// type at the closure's expression site. When the closure appears as a direct
// argument (or as the tail expression of a block), the expected type from the
// receiving function's trait bound provides parameter inference. However,
// wrapping the closure in a generic function — even an unbounded identity
// function — breaks this inference because generic functions introduce fresh
// type variables that are not constrained by the outer expected type.
//
// This directly impacts how the localized error system generates pre-check
// code:
// - IndependentBounds: the check function's supertrait has CONCRETE Fn
//   params (e.g. `Check: Fn(i32) -> String`), so the closure gets its
//   parameter types from the check function's bound directly.
// - DependentBounds/Unchecked: the check function has blanket/no bounds
//   (no useful Fn signature), and the outer builder setter's expected
//   type does NOT propagate backward through the generic call — E0282.
//   This is why DependentBounds uses the struct's type parameter directly
//   (not a fresh generic) and Unchecked uses a blanket impl.
//
// Expected errors:
// - E0282 for CASE 3 (unbounded identity function wrapping closure).
//
// Note: CASE 5 (let binding in block) also produces E0282 when tested
// in isolation, but the compiler aborts after the first E0282 in CASE 3
// so it does not appear in the stderr output.

struct Builder;
struct Builder2<T>(std::marker::PhantomData<T>);

impl Builder {
    fn each<IF: Fn() -> Vec<T>, T>(self, _f: IF) -> Builder2<T> {
        todo!()
    }
}
impl<T> Builder2<T> {
    fn key<KF: Fn(&T) -> K, K: Eq + std::hash::Hash>(self, _f: KF) -> Self {
        todo!()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct Counter {
    id: usize,
}

// BoundedSingleParam-style: supertrait provides CONCRETE Fn signature.
#[diagnostic::on_unimplemented(message = "wrong type")]
trait CheckTransform: Fn(i32) -> String {}
impl<T: Fn(i32) -> String> CheckTransform for T {}
fn check_transform<T: CheckTransform>(val: T) -> T {
    val
}

struct SimpleBuilder;
impl SimpleBuilder {
    fn transform<F: Fn(i32) -> String>(self, _f: F) -> Self {
        todo!()
    }
}

// Unbounded identity function — zero bounds on T.
fn identity<T>(val: T) -> T {
    val
}

fn main() {
    // CASE 1: Direct argument — WORKS.
    // The builder's `.key()` expects `KF: Fn(&Counter) -> K`, which provides
    // concrete parameter types to the closure.
    let _ = Builder
        .each(|| vec![Counter { id: 0 }])
        .key(|counter| counter.id);

    // CASE 2: BoundedSingleParam check function — WORKS.
    // `check_transform` has `T: CheckTransform` where `CheckTransform: Fn(i32) -> String`.
    // The supertrait provides CONCRETE Fn parameter types, so the closure can infer
    // its parameter type from the check function's bound alone.
    let _ = SimpleBuilder.transform(check_transform(|x| x.to_string()));

    // CASE 3: Unbounded identity function — FAILS.
    // `identity` has `fn<T>(T) -> T` with no Fn bound. The expected type from
    // `.key()` does NOT propagate backward through the generic function call to
    // constrain the closure's parameter types.
    let _ = Builder
        .each(|| vec![Counter { id: 0 }])
        .key(identity(|counter| counter.id)); //~ ERROR type annotations needed

    // CASE 4: Block with tail expression — WORKS.
    // The expected type from `.key()` flows into the block and reaches the
    // closure as the tail expression. Unrelated statements before the tail
    // do not interfere.
    let _ = Builder
        .each(|| vec![Counter { id: 0 }])
        .key({
            let _ = 42;
            |counter| counter.id
        });

    // CASE 5: Block with let binding — FAILS.
    // A `let` binding forces the closure's type to be resolved at the binding
    // site, before the block's expected type can provide inference.
    let _ = Builder
        .each(|| vec![Counter { id: 0 }])
        .key({
            let v = |counter| counter.id; //~ ERROR type annotations needed
            v
        });
}
