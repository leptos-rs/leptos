# Localized Error Reporting in `view!` Macro

UX improvement when working with the view! macro.

## Goals

- Errors in the view macro must be reported where they occur.
- Errors should state in their first line what the root issue is.
- A single type error should not cascade into 5+ confusing downstream errors.

## Constraints

- Each `#[component]` component can produce a companion module alongside the function name (type vs value namespace).
- Companion modules MUST NOT rely on `use super::*;` to get access to user-imported types, because `#[component]`s
  can be declared inside other functions (e.g. doc tests) and inner modules don't inherit the parent function's scope.
- Therefore: A companion module cannot use the component's full generics inside it, as they likely contain user-defined
  types. However, **structurally-stripped** generics can be used, as they won't reference user-defined types.

## Architecture Overview

Both `#[component]` and `#[slot]` use the same **companion module pattern**. For a component named `Foo`:

```
mod Foo { ... }       // companion module (type namespace)
fn Foo(props) { ... } // the component function (value namespace)
```

For a slot named `Bar`, the module is prefixed to avoid collision with the struct:

```
mod __Bar { ... }     // companion module
struct Bar { ... }    // the slot struct
```

The companion module contains:

- Per-prop **check traits** (`__Check_foo`) for clean UFCS error messages
- Per-prop **pass traits** (`__Pass_foo`) for `{error}` propagation via method syntax
- **`__CheckAllRequired`** trait for missing-prop detection
- **`__CheckMissing`** trait for `{error}` propagation
- **`__require_props()`** function as the entry point for required-prop checking
- **`__builder()`** function (slots only)

## The Two-Step Pre-Check + Required Check Mechanism

Every prop usage in `view!` goes through two pre-check steps plus required-prop checking.

### 1a. UFCS check trait (`__Check_foo`) — clean error message

**Purpose**: E0277 with `on_unimplemented` — works for ALL types including closures.

**Generated inside module**:

```rust
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid type for prop `foo` ...",
    note = "required: `Fn() -> bool`"
)]
pub trait __Check_foo {
    fn __check_foo(&self);
}
```

**Generated outside module** (bounded impl):

```rust
impl<T: Fn() -> bool> Foo::__Check_foo for T {
    fn __check_foo(&self) {}
}
```

**Called in view expansion** via UFCS:

```rust
let __value_foo = user_value;
<_ as Foo::__Check_foo>::__check_foo(&__value_foo);
```

When the bound fails, E0277 fires with the custom `on_unimplemented` message. For closures,
the compiler produces its own targeted diagnostics (E0271: "expected closure to return X, but
it returns Y") which are **more actionable** than our generic message.

### 1b. Method call pass trait (`__Pass_foo`) — `{error}` propagation

**Purpose**: E0599 → `{error}` type that suppresses downstream errors.

**Generated inside module**:

```rust
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid type for prop `foo` ...",
    note = "required: `Fn() -> bool`"
)]
pub trait __Pass_foo {
    fn __pass_foo(self) -> Self;
}
```

**Generated outside module** (bounded on `__Check_foo`):

```rust
impl<T: Foo::__Check_foo> Foo::__Pass_foo for T {
    fn __pass_foo(self) -> Self { self }
}
```

**Called in view expansion** via method syntax (requires `use Foo::__Pass_foo as _;`):

```rust
let __checked_foo = __value_foo.__pass_foo();
```

When the bound fails, E0599 fires and the expression type is `{error}`, which absorbs
all downstream errors in the builder chain.

**Why both steps are needed**: UFCS (step 1a) gives clean E0277 messages for all types
including closures, but does NOT produce `{error}` (the return type is resolved via
bidirectional type inference). Method syntax (step 1b) produces `{error}` for downstream
suppression, but E0599 ignores `on_unimplemented` for anonymous types (closures).

For **concrete/into/unbounded props**, blanket impls are generated for both traits
(`impl<T> __Check_foo for T` and `impl<T> __Pass_foo for T`), letting all types through —
the typed-builder setter handles type checking for these.

### 2. `__require_props()` (E0277 for missing props)

**Purpose**: Clear "missing required prop" error message.

```rust
pub fn __require_props<B: __CheckAllRequired>(_: &B) {}
```

`__CheckAllRequired` is implemented for the builder type only when all required type-state slots are
filled. Each required prop `foo` gets a marker trait `__required_Comp_foo`, implemented for `(T,)` but
not `()`. When a required prop is missing, E0277 fires with:
`missing required prop 'foo' on component 'Comp'`

### 3. `__check_missing()` (E0599 for `{error}` propagation)

**Purpose**: Suppress downstream errors when a required prop is missing.

`__CheckMissing` is implemented under the same bounds as `__CheckAllRequired`. When bounds aren't met,
`builder.__check_missing()` fails with E0599, producing `{error}` type that suppresses the final
`component_view()` and `.build()` errors.

**Important**: `__check_missing` must be called as a **method** (`builder.__check_missing()`), not via
UFCS. Method calls produce `{error}` on failure; UFCS calls produce E0277 which does NOT suppress
downstream errors.

## Prop Classification

Props are classified by `util::classify_prop()`:

| Classification       | Condition                                                                   | Check behavior                           |
|----------------------|-----------------------------------------------------------------------------|------------------------------------------|
| `BoundedSingleParam` | Bare generic with bounds, single param, bounds don't reference other params | Custom `on_unimplemented` + bounded impl |
| `PassThrough`        | Everything else (concrete, `into`, unbounded, multi-param bounds)           | Blanket impl, all types pass through     |

Examples:

- `fun: F` where `F: Fn() -> bool` → `BoundedSingleParam`
- `count: i32` → `PassThrough` (concrete)
- `label: String` with `#[prop(into)]` → `PassThrough` (into)
- `action: ServerAction<S>` where `S: ServerFn` → `PassThrough` (generic inside wrapper, needs structural bounds)

### Why Blanket PassThrough Impls Are Necessary

The `view!` macro expands without knowledge of the component's generics or prop classifications — it only
sees attribute names and values. It generates `__Check_*` / `__Pass_*` calls for ALL props uniformly.
Therefore, the `#[component]` macro must provide blanket impls for PassThrough props so these uniform
pre-check calls compile. Skipping pre-check traits for PassThrough props is not possible without breaking
the separation between `#[component]` (knows prop types) and `view!` (doesn't).

## Structural vs Behavioral Bounds

The props struct only carries **structural** bounds — those needed for the struct to be well-formed.
A generic param needs structural bounds when it appears inside another generic type in a field
(e.g., `ServerAction<S>` needs `S: ServerFn` on the struct). A bare generic param (`fun: F`) does NOT
need bounds on the struct.

**Behavioral** bounds (like `F: Fn() -> bool`) are deferred to the per-prop check traits, where they
produce better error messages.

`strip_non_structural_bounds()` in `util.rs` performs this separation.

## Error Behavior by Prop Kind

| Prop kind                 | Example         | Error 1 (clean)           | Error 2                  | Points to         |
|---------------------------|-----------------|---------------------------|--------------------------|-------------------|
| Concrete, expanded        | `count=42`      | E0308 (type mismatch)     | —                        | Value (`42`)      |
| Concrete, short form      | `flag`          | E0308 (type mismatch)     | —                        | Key (`flag`)      |
| Generic, bounded (named)  | `fun=true`      | E0277 (on_unimplemented)  | E0599 (on_unimplemented) | Value (`true`)    |
| Generic, bounded (closure)| `fun=\|\| true` | E0271/E0593 (targeted)    | E0599 (noisy)            | Value (`\|\| true`)|
| Generic, short form       | `fun`           | E0277 (on_unimplemented)  | E0599 (on_unimplemented) | Key (`fun`)       |
| `into` prop               | `label=vec![1]` | E0277 (From not impl)     | —                        | Value (`vec![1]`) |
| Missing required          | `<Comp/>`       | E0277 (on_unimplemented)  | —                        | Component name    |

## `{error}` Type Propagation

When a **method call** fails (E0599), the Rust compiler assigns the expression the special `{error}`
type. This type is compatible with everything — any subsequent method call or function call on it
silently succeeds at type-checking, producing no additional errors.

**Important**: Only E0599 (method not found) produces `{error}`. E0277 from UFCS does NOT — the
return type is resolved via bidirectional type inference even when the impl doesn't exist.

The two-step approach exploits this:

1. `<_ as __Check_foo>::__check_foo(&value)` fails → E0277 with clean message (but NO `{error}`)
2. `value.__pass_foo()` fails → E0599, expression is `{error}`
3. `builder.foo(__checked_foo)` uses `{error}` → builder type propagates `{error}`
4. `__require_props(&builder)` — absorbed by `{error}`
5. `builder.__check_missing()` — absorbed by `{error}`
6. `component_view(Comp, props)` — absorbed by `{error}`

Result: 2 errors total (clean E0277 + noisy E0599), all downstream suppressed.

## Span Strategy

- **Check/pass method names** (`__check_foo`, `__pass_foo`) are created with the **value span** (or
  key span for short-form). This localizes errors to the user's source code.
- **Check/pass trait names** (`__Check_foo`, `__Pass_foo`) use `Span::call_site()` to avoid polluting
  IDE navigation.
- **Component/slot name** in `__require_props` and `__check_missing` uses the original name span for
  missing-prop errors.
- **`delinked_path_from_node_name()`** replaces the last segment's span with `call_site()` for
  type-namespace usages (builder, checks), ensuring IDE ctrl+click navigates to the function, not the
  module.

## Code Organization

Shared logic lives in `leptos_macro/src/util.rs`:

- `classify_prop()` — determines `BoundedSingleParam` vs `PassThrough`
- `generate_module_checks()` — generates per-prop pass traits and impls
- `generate_module_required_check()` — generates required-prop checking traits
- `strip_non_structural_bounds()` — separates structural from behavioral bounds
- Various helpers: `type_contains_ident`, `collect_predicates_for_param`, `bounds_reference_other_params`, etc.

Call sites:

- `component.rs` calls with `module_name = display_name = ComponentName`
- `slot.rs` calls with `module_name = __SlotName`, `display_name = SlotName`

View-side code in `view/utils.rs`:

- `generate_pre_check_tokens()` — generates two-step UFCS check + method call
- `generate_pass_imports()` — generates `use Module::__Pass_foo as _;` imports
- `attr_check_idents()` — computes per-prop check identifiers

## Approaches That Don't Work for `{error}` Propagation

- **UFCS alone** (`<_ as Trait>::method(val)`): E0277 fires but the return type is resolved via
  Rust's bidirectional type inference (the builder setter constrains the type), so no `{error}`.
- **UFCS + associated type** (`<_ as Trait>::method(val) -> Self::Checked`): Same issue —
  the projection `<V as Trait>::Checked` gets resolved by inference from downstream usage.
- **`impl Trait` return**: Compiler resolves T through the function signature, bypassing `{error}`.
- **Local pass trait at call site**: Works perfectly (1 error!) but the prop-bound trait isn't accessible
  when the component is imported from another module.
- **GAT return type**: Changes the primary error from E0277 to E0599, losing `on_unimplemented`.
- **`on_unimplemented` on wrapper E0599**: When `impl<T: Bound> Trait for Wrapper<T>`, E0599 only
  mentions the unsatisfied bound as a note, NOT using `on_unimplemented` as the primary message.

## Current Approach

Two-step pre-check for bounded generic props:

1. Strip behavioral bounds from the Props struct (deferred to check traits)
2. UFCS check (`__Check_*`) for clean E0277 message (works for closures)
3. Method call (`__Pass_*`) for E0599 → `{error}` propagation

This gives 2 errors for wrong-type bounded generic props (clean first + noisy second),
with all downstream errors suppressed.

## Findings

### UFCS vs Method Syntax for Component Pre-Checks

Empirically verified with rustc nightly-2026-02-11.

#### UFCS `<_ as Trait>::method(val)` when bound fails

| Scenario                                                   | Error code | `on_unimplemented` shown?                           | `{error}` propagation?                    |
|------------------------------------------------------------|------------|-----------------------------------------------------|-------------------------------------------|
| Named type (e.g. `bool`, `String`)                         | E0277      | **Yes** — custom message is primary                 | **No** — return type resolved by inference |
| Closure, wrong return type (`\|\| true` for `Fn() -> i32`) | E0271      | **No** — compiler gives its own "expected X, got Y" | **No** — return type resolved by inference |
| Closure, wrong arity (`\|x\| 42` for `Fn() -> i32`)        | E0593      | **No** — compiler gives its own "takes N args"      | **No** — return type resolved by inference |
| Non-Fn type as closure                                     | E0277      | **Yes**                                             | **No** — return type resolved by inference |

**Critical finding**: UFCS does NOT produce `{error}` for the return type. Even with associated
types (`type Checked; fn pass(self) -> Self::Checked`), Rust's bidirectional type inference
resolves the concrete type from downstream usage (builder setter). This means UFCS alone cannot
suppress downstream errors.

#### Key insight: E0271/E0593 for closures are BETTER than custom messages

When a closure partially matches `Fn` (right trait family, wrong signature), the compiler produces targeted diagnostics:

- E0271: `"expected {closure} to return i32, but it returns bool"` — tells exact fix needed
- E0593: `"closure is expected to take 0 arguments, but it takes 1 argument"` — tells exact fix needed

These are **more actionable** than our generic `on_unimplemented` message (
`"{Self} is not a valid type for prop fun_b"`). The compiler's built-in closure diagnostics are superior for these
cases.

#### Method syntax `val.method()` when bound fails

| Scenario               | Error code | `on_unimplemented` shown?                                                          | `{error}` propagation? |
|------------------------|------------|------------------------------------------------------------------------------------|------------------------|
| Named type (`bool`)    | E0599      | **Yes** — custom message is primary                                                | **Yes**                |
| Closure (any mismatch) | E0599      | **No** — ugly default: `"method __pass_foo exists but trait bounds not satisfied"` | **Yes**                |

#### Why we need BOTH UFCS + method syntax (two-step approach)

Neither approach alone is sufficient:
- **UFCS only**: Clean messages for closures, but no `{error}` propagation → 3+ errors
- **Method only**: `{error}` propagation works, but ugly E0599 for closures
- **Both**: UFCS gives clean first error, method gives `{error}` → 2 errors total

The `__Check_*` trait (UFCS) is bounded directly on user bounds (`Fn() -> i32`).
The `__Pass_*` trait (method) is bounded on `__Check_*`, creating the `on_unimplemented` chain.

#### Span behavior in UFCS

For `<_ as Trait>::method(val)`:

- Named types: error points to `_` token (the inferred Self type)
- Closures: error points to the value expression

In proc macros with `quote_spanned!{value_span=> <_ as Path>::method(val)}`, the `_` token gets the value span, so the
error localizes to the source value expression.

#### Summary: two-step approach comparison

| Criterion               | Method only (old)         | UFCS only               | Two-step (current)                    |
|-------------------------|---------------------------|-------------------------|---------------------------------------|
| Named types             | 1 clean error             | 3 errors (clean first)  | 2 errors (both show on_unimplemented) |
| Closures (wrong return) | 1 ugly error              | 3 errors (clean first)  | 2 errors (clean first + noisy second) |
| Closures (wrong arity)  | 1 ugly error              | 3 errors (clean first)  | 2 errors (clean first + noisy second) |
| `{error}` propagation   | Yes                       | No                      | Yes (from method step)                |
| Downstream suppression  | Yes                       | No                      | Yes                                   |

## Test Coverage

Trybuild tests in `leptos_macro/tests/view/` with `.stderr` snapshots:

| Test  | Scenario                                   |
|-------|--------------------------------------------|
| 02-04 | Concrete props (correct usage)             |
| 05    | Missing required concrete prop             |
| 06    | Concrete prop wrong type                   |
| 07-08 | Generic props (correct usage)              |
| 09    | Generic prop wrong type (expanded form)    |
| 10    | Generic prop wrong type (short form)       |
| 11-12 | Multiple generic params (correct)          |
| 13    | Multiple generic params, first wrong type  |
| 14    | Multiple generic params, second wrong type |
| 37    | Slot (correct usage)                       |
| 38    | Slot generic prop wrong type               |

Run tests with:

```bash
cargo +nightly test -p leptos_macro --test view
```

Update `.stderr` snapshots with:

```bash
TRYBUILD=overwrite cargo +nightly test -p leptos_macro --test view
```
