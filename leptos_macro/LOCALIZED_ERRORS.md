# Localized Error Reporting in `view!` Macro

## Goals

- Errors in the view macro must be reported where they occur.
- Errors should state in their first line what the root issue is.
- A single error should not cascade into multiple other confusing downstream errors.

## Architecture

`#[component]`s use the **companion module pattern**:

```
fn Foo(props) { ... } // component function (value namespace)
mod Foo { ... }       // companion module (type namespace, no collision)
```

`#[slot]`s use the **companion module pattern**:

```
struct FooSlot { ... } // slot struct
mod __FooSlot { ... }  // companion module (prefixed to avoid type-namespace collision with slot struct)
```

The companion module contains a `Helper` struct — the central access point for the view macro — which provides
`builder()`, `presence()`, and per-prop `check_and_wrap_*()` methods. It also contains per-prop check traits, wrapper
structs, and the `PropPresence` for missing-prop detection.

### Why a Helper Struct?

The `Helper` struct exists because the view macro needs a **single object that carries all of
the component's generic type parameters** and routes every prop check through it:

```rust
let __helper = Component::__helper::<T, F>();     // Helper<T, F> with PhantomData
let __checked_a = __helper.check_and_wrap_a(val_a).extract_value();
let __checked_b = __helper.check_and_wrap_b(val_b).extract_value();
```

Because all calls target the same `__helper` instance, the compiler unifies `T` and `F` across
every call. This is critical for **cross-parameter inference** — e.g. inferring `T` in
`KF: Fn(&T) -> K` requires chaining through `IF: Fn() -> I` and `I: IntoIterator<Item = T>`,
predicates on *other* params. That chain only works when all predicates are visible in one impl
block on one type.

A struct is also the only Rust construct that supports **two impl blocks with different where
clauses** on the same generic parameters — the unbounded impl (structural bounds only, for
`IndependentBounds`/`DeferredToBuilder` props) and the bounded impl (all original predicates, for
`DependentBounds` props).


## Mechanism

### Per-Prop Type Checking (Two-Step Pre-Check)

Every prop goes through two steps. For **bounded generic props** (e.g., `F: Fn() -> bool`), the check trait has a
bounded impl. For **everything else** (concrete, `into`, unbounded), a blanket impl lets all types through, meaning that
the typed-builder setter handles type checking.

```rust
// Step 1 — Check trait: Clean E0277 with on_unimplemented.
#[diagnostic::on_unimplemented(message = "`{Self}` is not valid for prop `foo` ...")]
pub trait Check_foo: Fn() -> bool {}        // inside module (structurally-stripped)
impl<T: Fn() -> bool> Foo::Check_foo for T {} // outside module (can use behavioral bounds)

// Step 2 — Bounded extract_value() on wrapper: E0599 → {error} for downstream suppression.
impl<__T: Fn() -> bool> Wrap_foo<__T> {     // inside module
    pub fn extract_value(self) -> __T { self.0 }
}

// view! expansion:
let __checked_foo = __helper.check_and_wrap_foo(value).extract_value();
```

Both steps are needed because they produce different things. A failed trait bound (step 1)
produces E0277 with a clean message — but the compiler still resolves the function's return
type and continues type-checking downstream code. Only a failed method lookup (step 2)
produces `{error}` as the expression type, which absorbs all downstream builder chain errors.
Step 1 also enables the compiler's native closure diagnostics (E0271/E0593).

The above shows the **IndependentBounds** case — props whose bounds are self-contained (don't
reference other component type params). Two other classifications exist:

- **DependentBounds** (e.g. `KF: Fn(&T) -> K` where `T` comes from another param): The check
  method lives in a bounded `Helper` impl with ALL original where-clause predicates, giving
  the compiler the full predicate chain for cross-param closure inference. Uses a blanket
  `extract_value()` (no additional type gate).
- **DeferredToBuilder** (concrete types, `into` props, wrapped generics): Blanket impls throughout;
  type checking is deferred entirely to the TypedBuilder setter.

### Missing-Prop Detection (Prop Presence)

The per-prop type check (`check_and_wrap_*`) validates prop **values** — but for a missing
prop, no value exists, so the view macro never emits a `check_and_wrap_*` call for it. Without
a separate mechanism, missing required props would only be caught by TypedBuilder's internal
error types (e.g. `PropsBuilder_Error_Missing_required_field_foo`), which are confusing and
not under our control.

The `PropPresence` fills this gap. It tracks which prop setters were called via type-state
(`PhantomData` tuples), completely independent of actual prop values. Since it never receives
`{error}` values, its checks work regardless of type errors elsewhere.

Each required prop `foo` gets a marker trait `required_Comp_foo`, implemented for `Present`
but not `Absent`. When a required prop is missing (setter not called), its slot stays at `Absent`.

- **`require_props`** (inherent method with where-clause): Fires E0277 with clean
  `on_unimplemented` message listing all required props.
- **`check_missing`** (bounded inherent method): When a required prop's marker trait is
  unsatisfied, E0599 fires → builder becomes `{error}` → suppresses TypedBuilder's
  confusing `.build()` errors.

This mirrors the two-step pattern from type checking: `require_props` provides the clean
diagnostic (E0277), while `check_missing` produces `{error}` to suppress downstream noise
(E0599). Neither alone is sufficient.

`check_missing` takes the builder as an argument so that when its bounds fail, the builder
becomes `{error}` and TypedBuilder's internal error types (like
`PropsBuilder_Error_Missing_required_field_foo`) are absorbed.

### End-to-End Flow

Given `<Inner generic_fun=true>` where `concrete_i32` is also required and `generic_fun` requires a closure, not bool:

1. `__helper.check_and_wrap_generic_fun(true).extract_value()` → **E0277** (clean wrong-type) + **E0599** (`{error}`)
2. `__presence.generic_fun()` → marks present (independent of `{error}`)
3. `__presence.require_props()` → **E0277** (missing `concrete_i32`, because its slot is still `Absent`)
4. `builder.generic_fun({error})` → builder becomes `{error}` (method called on `{error}` input)
5. `__presence.check_missing(builder)` → **E0599** (bounded impl unavailable because `concrete_i32` slot is `Absent`) → builder becomes `{error}`
6. `{error}.build()` → absorbed
7. `component_view(Comp, {error})` → absorbed

Result: 4 errors — wrong-type (E0277 + E0599) + missing-prop (E0277 + E0599), all simultaneous.

## Constraints

The module boundary is the key design constraint:

- Components can be declared inside functions (e.g. doc tests), and inner modules don't inherit the parent function's
  scope. So the companion module MUST NOT rely on `use super::*;`.
- Therefore, only **structurally-stripped** generics can appear inside the module — lifetime bounds, `Sized`, and other
  compiler-intrinsic constraints are kept, while behavioral bounds (like `Fn() -> bool` or user trait bounds) are
  stripped because they may reference types not in scope.

This is why check trait definitions and wrapper structs live *inside* the module (using only structurally-stripped
generics), while their blanket impls — which carry behavioral bounds — live *outside* the module where user types
are in scope.

## Running Tests

```bash
cargo +nightly test -p leptos_macro --test view
cargo +nightly test -p leptos_macro --test compiler_assumptions
TRYBUILD=overwrite cargo +nightly test -p leptos_macro --test view  # update snapshots
```
