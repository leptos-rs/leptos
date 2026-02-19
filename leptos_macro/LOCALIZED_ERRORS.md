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

- Per-prop **check traits** (`__Check_foo`) with both `__check_foo(&self)` for clean UFCS error messages
  and `__pass_foo(self)` for `{error}` propagation via method syntax
- **`__PresenceBuilder`** — lightweight type-state builder tracking which props are present, immune to
  `{error}` contamination. Also has a bounded inherent `__check_missing()` method for `{error}` propagation.
- **`__CheckPresence`** trait with `__require_props(&self)` for missing-prop detection (on presence builder)
- **`__builder()`** function that constructs the props builder

## The Two-Step Pre-Check + Required Check Mechanism

Every prop usage in `view!` goes through two pre-check steps plus required-prop checking.

### 1a. UFCS check (`__check_foo`) — clean error message

**Purpose**: E0277 with `on_unimplemented` — works for ALL types including closures.

**Generated inside module** (single trait with both methods):

```rust
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid type for prop `foo` ...",
    note = "required: `Fn() -> bool`"
)]
pub trait __Check_foo {
    fn __check_foo(&self);
    fn __pass_foo(self) -> Self;
}
```

**Generated outside module** (bounded impl):

```rust
impl<T: Fn() -> bool> Foo::__Check_foo for T {
    fn __check_foo(&self) {}
    fn __pass_foo(self) -> Self { self }
}
```

**Called in view expansion** via UFCS:

```rust
let __value_foo = user_value;
< _ as Foo::__Check_foo>::__check_foo( & __value_foo);
```

When the bound fails, E0277 fires with the custom `on_unimplemented` message. For closures,
the compiler produces its own targeted diagnostics (E0271: "expected closure to return X, but
it returns Y") which are **more actionable** than our generic message.

### 1b. Method call (`__pass_foo`) — `{error}` propagation

**Purpose**: E0599 → `{error}` type that suppresses downstream errors.

**Called in view expansion** via method syntax (requires `use Foo::__Check_foo as _;`):

```rust
let __checked_foo = __value_foo.__pass_foo();
```

Both `__check_foo` and `__pass_foo` live on the same `__Check_foo` trait. When the bound
fails, the method call `__pass_foo()` produces E0599, and the expression type is `{error}`,
which absorbs all downstream errors in the builder chain.

**Why both steps are needed**: UFCS (step 1a) gives clean E0277 messages for all types
including closures, but does NOT produce `{error}` (the return type is resolved via
bidirectional type inference). Method syntax (step 1b) produces `{error}` for downstream
suppression, but E0599 ignores `on_unimplemented` for anonymous types (closures).

For **concrete/into/unbounded props**, blanket impls are generated
(`impl<T> __Check_foo for T { ... }`), letting all types through —
the typed-builder setter handles type checking for these.

### 2. `__require_props()` via Presence Builder (E0277 for missing props)

**Purpose**: Clear "missing required prop" error message, independent of `{error}` contamination.

The **presence builder** (`__PresenceBuilder`) tracks which props are present via type-state (PhantomData
tuples), completely independent of actual prop values. Since it never receives `{error}` values from
wrong-type props, its `__require_props` check works regardless of type errors.

**Generated inside module**:

```rust
pub struct __PresenceBuilder<S>(PhantomData<S>);

pub fn __presence() -> __PresenceBuilder<((), (), ...)> { ... }

impl<F0, F1, ...> __PresenceBuilder<(F0, F1, ...)> {
    pub fn foo(self) -> __PresenceBuilder<(((),), F1, ...)> { ... }
    pub fn bar(self) -> __PresenceBuilder<(F0, ((),), ...)> { ... }
}

pub trait __CheckPresence {
    fn __require_props(&self);
}
```

**Generated outside module**:

```rust
impl<F0: __required_Comp_foo, F1: __required_Comp_bar, ...>
    __CheckPresence for Comp::__PresenceBuilder<(F0, F1, ...)>
{
    fn __require_props(&self) {}
}
```

**Called in view expansion** via UFCS:

```rust
let __presence = Foo::__presence();
let __presence = __presence.foo();  // mark foo as present
let __presence = __presence.children();  // mark children as present
<_ as Foo::__CheckPresence>::__require_props(&__presence);
```

Each required prop `foo` gets a marker trait `__required_Comp_foo`, implemented for `(T,)` but
not `()`. When a required prop is missing (setter not called), its type-state stays at `()`, and
E0277 fires with: `missing required prop 'foo' on component 'Comp'`

### 3. `__check_missing()` on `__PresenceBuilder` (E0599 for `{error}` propagation)

**Purpose**: Suppress downstream errors when a required prop is missing.

`__check_missing` is a **bounded inherent method** on `__PresenceBuilder`, NOT a trait method on the
real builder. This eliminates the dependency on TypedBuilder's internal type-state representation.

**Generated inside module** (bounded inherent impl):

```rust
impl<__F0: __required_Comp_foo, __F1, ...>
    __PresenceBuilder<(__F0, __F1, ...)>
{
    pub fn __check_missing<__B>(self, builder: __B) -> __B {
        builder
    }
}
```

The bounds use the same marker traits as `__CheckPresence`. When a required prop is missing, its
type-state slot is `()` (which doesn't implement the marker trait), so `__check_missing` is unavailable
→ E0599 → `{error}` on the builder, suppressing `.build()` and `component_view()`.

**Called in view expansion**:

```rust
let __props_builder = __presence.__check_missing(__props_builder);
let props = __props_builder.build();
```

**Why this is on `__PresenceBuilder` and not the real builder**: The real builder's type-state is an
internal implementation detail of TypedBuilder (undocumented tuple structure). By using the presence
builder's type-state (which we fully control), we avoid depending on TypedBuilder internals. The
presence builder is also immune to `{error}` contamination from wrong-type props, so its bounded
inherent method works reliably regardless of type errors.

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

### Why Blanket Impls Are Necessary for `PassThrough`-Classified Props

The `view!` macro expands without knowledge of the component's generics or prop classifications — it only
sees attribute names and values. It generates `__check_*` / `__pass_*` calls for ALL props uniformly.
Therefore, the `#[component]` macro must provide blanket impls for `PassThrough`-classified props so these
uniform pre-check calls compile. Skipping pre-check traits for these props is not possible without breaking
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

| Prop kind                  | Example         | Error 1 (clean)          | Error 2       | Points to           |
|----------------------------|-----------------|--------------------------|---------------|---------------------|
| Concrete, expanded         | `count=42`      | E0308 (type mismatch)    | —             | Value (`42`)        |
| Concrete, short form       | `flag`          | E0308 (type mismatch)    | —             | Key (`flag`)        |
| Generic, bounded (named)   | `fun=true`      | E0277 (on_unimplemented) | E0599 (noisy) | Value (`true`)      |
| Generic, bounded (closure) | `fun=\|\| true` | E0271/E0593 (targeted)   | E0599 (noisy) | Value (`\|\| true`) |
| Generic, short form        | `fun`           | E0277 (on_unimplemented) | E0599 (noisy) | Key (`fun`)         |
| `into` prop                | `label=vec![1]` | E0277 (From not impl)    | —             | Value (`vec![1]`)   |
| Missing required           | `<Comp/>`       | E0277 (on_unimplemented) | E0599 (noisy, on `__PresenceBuilder`) | Component name      |

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
4. `__presence.__check_missing(__props_builder)` — absorbed because `{error}` builder is accepted
5. `component_view(Comp, props)` — absorbed by `{error}`

Meanwhile, `<_ as __CheckPresence>::__require_props(&__presence)` on the presence builder runs
independently and is NOT affected by `{error}`.

Result: wrong-type errors (clean E0277 + noisy E0599) + missing-prop errors (E0277 from presence
builder) all shown simultaneously.

## Error Priority

Wrong-type and missing-prop errors are shown **simultaneously** thanks to the presence builder.
The presence builder tracks prop presence via type-state without receiving actual values, so it
is immune to `{error}` contamination from wrong-type props.

For example, given `<Inner generic_fun=true>` where `concrete_i32` is also required:

**Pre-checks** (independent of builder):
1. `<_ as __Check_generic_fun>::__check_generic_fun(&true)` → E0277 (clean wrong-type message)
2. `true.__pass_generic_fun()` → E0599, expression is `{error}`

**Presence tracking** (independent of {error}):
3. `__presence.generic_fun()` → marks generic_fun present
4. `__presence.children()` → marks children present
5. `<_ as __CheckPresence>::__require_props(&__presence)` → E0277 for missing `concrete_i32`

**Real builder** (contaminated by {error}):
6. `builder.generic_fun({error})` → builder is `{error}`
7. `__presence.__check_missing({error})` → E0599 because presence has missing field bounds
8. `{error}.build()` → absorbed
9. `component_view(Comp, {error})` → absorbed

The user sees **4 errors**: wrong-type (E0277 + E0599), missing-prop from `__require_props` (E0277),
and missing-prop from `__check_missing` (E0599), all at once. In the pure missing-prop case (no
wrong-type), the user sees **2 errors**: clean E0277 + noisy E0599.

When **multiple** wrong-type props are present simultaneously, each produces its own independent
error pair (E0277 + E0599), since the pre-checks happen before the builder chain. Multiple
missing props also produce independent E0277 errors from `__require_props`.

## Span Strategy

- **Check/pass method names** (`__check_foo`, `__pass_foo`) are created with the **value span** (or
  key span for short-form). This localizes errors to the user's source code.
- **Check trait names** (`__Check_foo`) use `Span::call_site()` to avoid polluting IDE navigation.
- **Component/slot name** in `__require_props` and `__check_missing` uses the original name span for
  missing-prop errors.
- **`delinked_path_from_node_name()`** replaces the last segment's span with `call_site()` for
  type-namespace usages (builder, checks), ensuring IDE ctrl+click navigates to the function, not the
  module.

## Code Organization

Shared logic lives in `leptos_macro/src/util.rs`:

- `classify_prop()` — determines `BoundedSingleParam` vs `PassThrough`
- `generate_module_checks()` — generates per-prop check traits and impls
- `generate_module_required_check()` — generates per-required-prop marker traits with `on_unimplemented`
- `generate_module_presence_check()` — generates `__PresenceBuilder`, `__CheckPresence`, and bounded `__check_missing`
- `strip_non_structural_bounds()` — separates structural from behavioral bounds
- Various helpers: `type_contains_ident`, `collect_predicates_for_param`, `bounds_reference_other_params`, etc.

Call sites:

- `component.rs` calls with `module_name = display_name = ComponentName`, `kind = "component"`
- `slot.rs` calls with `module_name = __SlotName`, `display_name = SlotName`, `kind = "slot"`

Error messages use the `kind` parameter for context-appropriate wording:

- Component errors: `"missing required prop 'foo' on component 'Bar'"`
- Slot errors: `"missing required prop 'foo' on slot 'Bar'"`

View-side code in `view/utils.rs`:

- `generate_pre_check_tokens()` — generates two-step UFCS check + method call
- `generate_check_imports()` — generates `use Module::__Check_foo as _;` imports
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

## Why `#[diagnostic::do_not_recommend]` Doesn't Help Here

`#[diagnostic::do_not_recommend]` is designed for cases where **multiple trait impls compete** and
you want to deprioritize one in favor of another (e.g., stdlib's
`impl<T: Display> From<T> for String` is marked `do_not_recommend` so the compiler suggests more
specific `From` impls first).

In our generated code, this attribute doesn't help for several reasons:

- **Each trait has exactly one impl per type.** There is no alternative impl to deprioritize in
  favor of. `do_not_recommend` on a single impl just suppresses it with nothing better to show.
- **On bounded `__Check_*` impls, it suppresses `on_unimplemented`.** The clean E0277 message
  (from step 1a) is our primary user-facing error. Adding `do_not_recommend` hides it, leaving
  only the noisy E0599 from step 1b as the first error.
- **For closures, it suppresses Rust's native diagnostics.** E0271 ("expected closure to return X,
  but it returns Y") and E0593 ("closure takes N arguments") are more actionable than any custom
  message we could provide. `do_not_recommend` would suppress these too.
- **On `impl Props for ...`, it has no effect.** Each generated props struct has exactly one `Props`
  impl — there's nothing to deprioritize.

## Current Approach

Two-step pre-check for bounded generic props:

1. Strip behavioral bounds from the Props struct (deferred to check traits)
2. UFCS check (`__check_*`) for clean E0277 message (works for closures)
3. Method call (`__pass_*`) for E0599 → `{error}` propagation

This gives 2 errors for wrong-type bounded generic props (clean first + noisy second),
with all downstream errors suppressed.

## Findings

### UFCS vs Method Syntax for Component Pre-Checks

Empirically verified with rustc nightly-2026-02-11.

#### UFCS `<_ as Trait>::method(val)` when bound fails

| Scenario                                                   | Error code | `on_unimplemented` shown?                           | `{error}` propagation?                     |
|------------------------------------------------------------|------------|-----------------------------------------------------|--------------------------------------------|
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
| Named type (`bool`)    | E0599      | **No** — default: `"method pass exists for type bool, but trait bounds not satisfied"` | **Yes**                |
| Closure (any mismatch) | E0599      | **No** — default: `"method pass exists for closure, but trait bounds not satisfied"` | **Yes**                |

#### Why we need BOTH UFCS + method syntax (two-step approach)

Neither approach alone is sufficient:

- **UFCS only**: Clean messages for closures, but no `{error}` propagation → 3+ errors
- **Method only**: `{error}` propagation works, but ugly E0599 for closures
- **Both**: UFCS gives clean first error, method gives `{error}` → 2 errors total

Both `__check_*` (UFCS) and `__pass_*` (method) live on the same `__Check_*` trait,
bounded directly on user bounds (`Fn() -> i32`).

#### Span behavior in UFCS

For `<_ as Trait>::method(val)`:

- Named types: error points to `_` token (the inferred Self type)
- Closures: error points to the value expression

In proc macros with `quote_spanned!{value_span=> <_ as Path>::method(val)}`, the `_` token gets the value span, so the
error localizes to the source value expression.

#### Summary: two-step approach comparison

| Criterion               | Method only (old) | UFCS only              | Two-step (current)                    |
|-------------------------|-------------------|------------------------|---------------------------------------|
| Named types             | 1 clean error     | 3 errors (clean first) | 2 errors (clean first + noisy second) |
| Closures (wrong return) | 1 ugly error      | 3 errors (clean first) | 2 errors (clean first + noisy second) |
| Closures (wrong arity)  | 1 ugly error      | 3 errors (clean first) | 2 errors (clean first + noisy second) |
| `{error}` propagation   | Yes               | No                     | Yes (from method step)                |
| Downstream suppression  | Yes               | No                     | Yes                                   |

## Test Coverage

Trybuild tests in `leptos_macro/tests/view/` with `.stderr` snapshots:

| Test  | Scenario                                       |
|-------|------------------------------------------------|
| 02-04 | Concrete props (correct, wrong type)           |
| 05    | Missing required concrete prop                 |
| 06    | Concrete prop wrong type (multiple props)      |
| 07-08 | Generic props (correct, missing)               |
| 09    | Generic prop wrong type (expanded form)        |
| 10    | Generic prop wrong type (short form)           |
| 11-12 | Multiple generic params (correct)              |
| 13    | Multiple generic params, first wrong type      |
| 14    | Multiple generic params, second wrong type     |
| 15    | Children missing                               |
| 16    | Children FnOnce instead of Fn                  |
| 17-30 | Prop attributes (optional, default, into, etc) |
| 31-33 | Builder syntax                                 |
| 34-36 | Let syntax                                     |
| 37    | Slot (correct usage)                           |
| 38    | Slot generic prop wrong type                   |
| 39    | Raw identifier                                 |
| 40-41 | Renamed import of component                    |
| 42    | Multiple missing required props                |
| 43    | Multiple wrong-type props                      |
| 44    | Wrong type + missing prop (shown simultaneously) |
| 45    | Only optional props (should compile)             |
| 46    | Slot missing required prop                       |
| 47    | Lifetime parameterized component                 |
| 48    | Multiple components same prop names              |

### Compiler Assumption Tests

Trybuild tests in `leptos_macro/tests/compiler_assumptions/` pin the undocumented rustc
behaviors that the two-step pre-check relies on:

| Test | Assumption                                            | Verified by                        |
|------|-------------------------------------------------------|------------------------------------|
| 01   | UFCS does NOT produce `{error}` type                  | Both E0277 and E0308 appear        |
| 02   | Method calls DO produce `{error}` type                | Only E0599 appears, no E0308       |
| 03   | E0599 does NOT show `on_unimplemented` for closures   | No custom message in E0599 output  |
| 04   | UFCS with concrete tuple pattern matching produces E0277 with `on_unimplemented` | Structural mismatch = 1 clean E0277 |
| 05   | `{error}` from method call is absorbed by UFCS call   | No additional errors from UFCS     |

If any of these fail after a nightly update, the error localization strategy may need revision.

Run tests with:

```bash
cargo +nightly test -p leptos_macro --test view
cargo +nightly test -p leptos_macro --test compiler_assumptions
```

Update `.stderr` snapshots with:

```bash
TRYBUILD=overwrite cargo +nightly test -p leptos_macro --test view
TRYBUILD=overwrite cargo +nightly test -p leptos_macro --test compiler_assumptions
```

## Tested Hypotheses

### Hypothesis A: `label` attribute on `on_unimplemented` for check traits

**Tested**: 2026-02-18 with rustc nightly-2026-02-11

**Change**: Added `label = "this value does not satisfy the required trait bounds"` to the
`#[diagnostic::on_unimplemented]` attribute on `__Check_foo` traits in `generate_module_checks()`.

**Result**: **Rejected.** The `label` attribute controls the `^^^^` annotation text in E0277. It
replaced the compiler default (e.g., `the trait Fn() is not implemented for bool`) with our custom
text, pushing the specific trait info to a `= help:` line below.

Before:

```
10 |             <Inner concrete_i32=42 generic_fun=true>
   |                                                ^^^^ the trait `Fn()` is not implemented for `bool`
   |
   = note: required: `Fn() -> bool`
```

After:

```
10 |             <Inner concrete_i32=42 generic_fun=true>
   |                                                ^^^^ this value does not satisfy the required trait bounds
   |
   = help: the trait `Fn()` is not implemented for `bool`
   = note: required: `Fn() -> bool`
```

**Why rejected**: The compiler default is more informative at the point where the user's eyes land.
`the trait Fn() is not implemented for bool` tells you *what's wrong*. The custom label
`this value does not satisfy the required trait bounds` is vague and pushes the useful info one line
down. The first line of the `message` already provides the high-level context (`"bool" is not a valid
type for prop "generic_fun"`), so the inline label should give the specific *why* — which the compiler
default already does well.

### Hypothesis B: Remove `__check_missing()` method call for missing props

**Tested**: 2026-02-18 with rustc nightly-2026-02-11

**Change**: Removed `fn __check_missing(self) -> Self` from the `__CheckMissing` trait and all call
sites. The goal was to eliminate the noisy E0599 ("method `__check_missing` exists but trait bounds
not satisfied") for missing-prop errors, leaving only the clean E0277 from `__require_props`.

**Hypothesis**: Without `__check_missing()`, TypedBuilder's `.build()` error might be tolerable,
producing a simple "missing field" error.

**Result**: **Rejected.** Removing `__check_missing()` exposed TypedBuilder's own `.build()` errors,
which are worse:

| Test                      | Before (with `__check_missing`) | After (without)                                        |
|---------------------------|---------------------------------|--------------------------------------------------------|
| 05 (concrete missing)     | E0277 + E0599 = 2 errors        | E0277 + warning + E0061 = 2 errors + 1 warning         |
| 08 (generic missing)      | E0277 + E0599 = 2 errors        | E0277 + warning + E0061 = 2 errors + 1 warning         |
| 15 (children missing)     | E0277 + E0599 = 2 errors        | E0277 + warning + E0061 = 2 errors + 1 warning         |
| 42 (3 missing)            | 3× E0277 + E0599 = 4 errors     | 3× E0277 + warning + E0061 = 4 errors + 1 warning      |
| 44 (wrong type + missing) | 3 errors (type + missing)       | 2 errors (unchanged — `{error}` still suppresses)      |
| 46 (slot missing)         | E0277 + E0599 = 2 errors        | E0277 + warning + E0061 + E0282 = 3 errors + 1 warning |

Key problems with the TypedBuilder `.build()` errors:

- **Deprecation warning**: TypedBuilder marks `.build()` as deprecated when required fields are missing,
  producing a `use of deprecated method` warning with internal type names.
- **E0061 with ugly type names**: The error says `argument #1 of type
  InnerPropsBuilder_Error_Missing_required_field_concrete_bool is missing` and suggests
  `<Inner(/* InnerPropsBuilder_Error_Missing_required_field_concrete_bool */)/>`.
- **Extra E0282 for slots**: The slot case gains an additional "type annotations needed" error.

**Conclusion**: `__check_missing()` serves as a **firewall** that prevents TypedBuilder's internal
error mechanisms from leaking through. The E0599 from `__check_missing()` is noisy, but it's a single
error and it suppresses all TypedBuilder errors. Removing it makes the output strictly worse.

**Follow-up**: `__check_missing()` was later moved from a trait method on the real builder (`__CheckMissing`
trait) to a **bounded inherent method** on `__PresenceBuilder`. This eliminates the dependency on
TypedBuilder's internal type-state representation. The E0599 noise still occurs, but it now references
`__PresenceBuilder<((), ...)>` types (which we control) instead of `PropsBuilder<(TypedBuilderInternals)>`.

## Breaking Changes

- **`component_props_builder` removed from public API**: The `view!` macro now uses
  `ComponentName::__builder()` (resolves to the companion module) instead of
  `component_props_builder(&ComponentName)`. If any user code called `component_props_builder`
  directly, this is a breaking change.
- **`component::*` restricted in prelude**: `leptos/src/lib.rs` changed from re-exporting
  `component::*` to specific exports, preventing internal types (`NoProps`, `EmptyPropsBuilder`,
  etc.) from leaking into user scope.
