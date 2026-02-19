# Localized Error Reporting in `view!` Macro

## Goals

- Errors in the view macro must be reported where they occur.
- Errors should state in their first line what the root issue is.
- A single type error should not cascade into 5+ confusing downstream errors.

## Constraints

- Each `#[component]` produces a companion module alongside the function (type vs value namespace).
- Companion modules MUST NOT rely on `use super::*;` — components can be declared inside functions
  (e.g. doc tests), and inner modules don't inherit the parent function's scope.
- Therefore the module cannot use the component's full generics (which may reference user types).
  Only **structurally-stripped** generics (no user-type references) can appear inside the module.

## Architecture

Both `#[component]` and `#[slot]` use the **companion module pattern**:

```
mod Foo { ... }       // companion module (type namespace) — components
mod __Bar { ... }     // companion module — slots (prefixed to avoid struct collision)
fn Foo(props) { ... } // component function (value namespace)
struct Bar { ... }    // slot struct
```

The companion module contains:

- Per-prop **check traits** (`__Check_foo`) with `__check_foo(&self)` for clean UFCS error
  messages and `__pass_foo(self)` for `{error}` propagation via method syntax
- **`__PresenceBuilder`** — lightweight type-state builder tracking which props are present,
  immune to `{error}` contamination. Has a bounded inherent `__check_missing()` method for
  `{error}` propagation.
- **`__CheckPresence`** trait with `__require_props(&self)` for missing-prop detection
- **`__builder()`** function that constructs the props builder

### Key Files

Shared logic in `leptos_macro/src/util.rs`:

- `classify_prop()` — determines `BoundedSingleParam` vs `PassThrough`
- `generate_module_checks()` — per-prop check traits and impls
- `generate_module_required_check()` — per-required-prop marker traits with `on_unimplemented`
- `generate_module_presence_check()` — `__PresenceBuilder`, `__CheckPresence`, `__check_missing`
- `strip_non_structural_bounds()` — separates structural from behavioral bounds

Call sites:

- `component.rs` — `module_name = display_name = ComponentName`, `kind = "component"`
- `slot.rs` — `module_name = __SlotName`, `display_name = SlotName`, `kind = "slot"`

View-side code in `view/utils.rs`:

- `generate_pre_check_tokens()` — two-step UFCS check + method call
- `generate_check_imports()` — `use Module::__Check_foo as _;` imports
- `attr_check_idents()` — per-prop check identifiers

## Mechanism

### Per-Prop Type Checking (Two-Step Pre-Check)

Every prop goes through two steps. For **bounded generic props** (e.g., `F: Fn() -> bool`),
the check trait has a bounded impl. For **everything else** (concrete, `into`, unbounded),
a blanket impl lets all types through — the typed-builder setter handles type checking.

**Step 1 — UFCS check** (`__check_foo`): Clean E0277 with `on_unimplemented`.

```rust
// Generated trait (inside module):
#[diagnostic::on_unimplemented(message = "`{Self}` is not valid for prop `foo` ...")]
pub trait __Check_foo {
    fn __check_foo(&self);
    fn __pass_foo(self) -> Self;
}

// Generated impl (outside module, bounded):
impl<T: Fn() -> bool> Foo::__Check_foo for T { ... }

// View expansion (UFCS):
<_ as Foo::__Check_foo>::__check_foo( & value);
```

When the bound fails, E0277 fires with the custom message. For closures, the compiler
produces its own targeted diagnostics (E0271/E0593) which are more actionable.

**Step 2 — Method call** (`__pass_foo`): E0599 → `{error}` type for downstream suppression.

```rust
// View expansion (method syntax, requires `use Foo::__Check_foo as _;`):
let __checked_foo = value.__pass_foo();
```

When the bound fails, E0599 fires and the expression type becomes `{error}`, which absorbs
all downstream builder chain errors.

**Why both steps are needed**:

| Criterion             | UFCS only           | Method only     | Both (current)         |
|-----------------------|---------------------|-----------------|------------------------|
| Clean error message   | Yes (all types)     | No (E0599 only) | Yes (from UFCS step)   |
| Closure diagnostics   | Yes (E0271/593)     | No              | Yes (from UFCS step)   |
| `{error}` propagation | No                  | Yes             | Yes (from method step) |
| Total errors per prop | 3+ (no suppression) | 1 (ugly)        | 2 (clean + noisy)      |

> UFCS does NOT produce `{error}` — Rust's bidirectional type inference resolves the return
> type even when the impl doesn't exist. Only method syntax (E0599) produces `{error}`.

> E0599 does NOT show `on_unimplemented` for any type (named or anonymous). This is why the
> UFCS check is needed as the first error.

### Missing-Prop Detection (Presence Builder)

The **presence builder** (`__PresenceBuilder`) tracks which props are present via type-state
(`PhantomData` tuples), completely independent of actual prop values. Since it never receives
`{error}` values, its checks work regardless of type errors.

Each required prop `foo` gets a marker trait `__required_Comp_foo`, implemented for `(T,)`
but not `()`. When a required prop is missing (setter not called), its slot stays at `()`.

**`__require_props`** (via `__CheckPresence` trait, UFCS): Fires E0277 with clean
`on_unimplemented` message listing all required props.

**`__check_missing`** (bounded inherent method on `__PresenceBuilder`): When a required
prop's marker trait is unsatisfied, E0599 fires → builder becomes `{error}` →
suppresses TypedBuilder's confusing `.build()` errors.

```rust
let __props_builder = __presence.__check_missing(__props_builder);
let props = __props_builder.build();
```

**Why `__check_missing` takes the builder**: TypedBuilder enforces required fields for direct
builder usage (`Props::builder().field(val).build()`). When the view macro omits a prop, the
setter is never called, so `.build()` fails with E0061 and deprecation warnings referencing
internal types like `PropsBuilder_Error_Missing_required_field_foo`. The `{error}` from
`__check_missing` absorbs these. Making all TypedBuilder fields default to `unreachable!()`
would break direct builder usage (runtime panic instead of compile error).

### End-to-End Flow

Given `<Inner generic_fun=true>` where `concrete_i32` is also required:

1. `<_ as __Check_generic_fun>::__check_generic_fun(&true)` → **E0277** (clean wrong-type)
2. `true.__pass_generic_fun()` → **E0599**, expression is `{error}`
3. `__presence.generic_fun()` → marks present (independent of {error})
4. `<_ as __CheckPresence>::__require_props(&__presence)` → **E0277** (missing `concrete_i32`)
5. `builder.generic_fun({error})` → builder is `{error}`
6. `__presence.__check_missing({error})` → **E0599** (presence bounds fail)
7. `{error}.build()` → absorbed
8. `component_view(Comp, {error})` → absorbed

Result: 4 errors — wrong-type (E0277 + E0599) + missing-prop (E0277 + E0599), all simultaneous.

## Prop Classification

Props are classified by `util::classify_prop()`:

| Classification       | Condition                                                                   | Check behavior                           |
|----------------------|-----------------------------------------------------------------------------|------------------------------------------|
| `BoundedSingleParam` | Bare generic with bounds, single param, bounds don't reference other params | Custom `on_unimplemented` + bounded impl |
| `PassThrough`        | Everything else (concrete, `into`, unbounded, multi-param)                  | Blanket impl, all types pass through     |

The `view!` macro generates `__check_*`/`__pass_*` calls for ALL props uniformly (it doesn't
know prop types). Blanket impls for `PassThrough` props ensure these compile.

## Structural vs Behavioral Bounds

The props struct carries only **structural** bounds — those needed for well-formedness (e.g.,
`ServerAction<S>` needs `S: ServerFn`). Bare generic params (`fun: F`) don't need bounds on
the struct. **Behavioral** bounds (like `F: Fn() -> bool`) are deferred to check traits for
better error messages. `strip_non_structural_bounds()` performs this separation.

## Error Behavior by Prop Kind

| Prop kind                  | Example         | Error 1 (clean)          | Error 2       | Points to      |
|----------------------------|-----------------|--------------------------|---------------|----------------|
| Concrete, expanded         | `count=42`      | E0308 (type mismatch)    | —             | Value (`42`)   |
| Concrete, short form       | `flag`          | E0308 (type mismatch)    | —             | Key (`flag`)   |
| Generic, bounded (named)   | `fun=true`      | E0277 (on_unimplemented) | E0599 (noisy) | Value (`true`) |
| Generic, bounded (closure) | `fun=\|\| true` | E0271/E0593 (targeted)   | E0599 (noisy) | Value          |
| Generic, short form        | `fun`           | E0277 (on_unimplemented) | E0599 (noisy) | Key (`fun`)    |
| `into` prop                | `label=vec![1]` | E0277 (From not impl)    | —             | Value          |
| Missing required           | `<Comp/>`       | E0277 (on_unimplemented) | E0599 (noisy) | Component name |
| Duplicate prop             | `foo=1 foo=2`   | rstml parse error        | —             | 2nd occurrence |

## Duplicate Prop Detection

Caught at two levels:

1. **rstml parser** (primary): Detects duplicate attribute names before the view macro runs.
2. **View macro** (defense-in-depth): `component_builder.rs` and `slot_helper.rs` maintain a
   `HashSet<String>` and emit `compile_error!` if a duplicate slips through.

## Actionable Error Notes

- **Fn-bounded props**: Note includes `"required: \`Fn() -> bool\` — a closure or function reference"`.
- **Missing required props**: Note lists all required props: `"required props: [\`foo\`, \`bar\`]"`.

## Span Strategy

- **Check/pass method names** (`__check_foo`, `__pass_foo`): created with the **value span** (or key span for
  short-form), localizing errors to user source.
- **Check trait names** (`__Check_foo`): `Span::call_site()` to avoid polluting IDE navigation.
- **Component/slot name** in `__require_props`/`__check_missing`: original name span.
- **`delinked_path_from_node_name()`**: Replaces last segment span with `call_site()` for type-namespace usages, so IDE
  ctrl+click navigates to the function, not the module.

## Test Coverage

### View Macro Tests (`leptos_macro/tests/view/`)

| Test  | Scenario                                         |
|-------|--------------------------------------------------|
| 02-04 | Concrete props (correct, wrong type)             |
| 05    | Missing required concrete prop                   |
| 06    | Concrete prop wrong type (multiple props)        |
| 07-08 | Generic props (correct, missing)                 |
| 09    | Generic prop wrong type (expanded form)          |
| 10    | Generic prop wrong type (short form)             |
| 11-12 | Multiple generic params (correct)                |
| 13    | Multiple generic params, first wrong type        |
| 14    | Multiple generic params, second wrong type       |
| 15    | Children missing                                 |
| 16    | Children FnOnce instead of Fn                    |
| 17-30 | Prop attributes (optional, default, into, etc)   |
| 31-33 | Builder syntax                                   |
| 34-36 | Let syntax                                       |
| 37    | Slot (correct usage)                             |
| 38    | Slot generic prop wrong type                     |
| 39    | Raw identifier                                   |
| 40-41 | Renamed import of component                      |
| 42    | Multiple missing required props                  |
| 43    | Multiple wrong-type props                        |
| 44    | Wrong type + missing prop (shown simultaneously) |
| 45    | Only optional props (should compile)             |
| 46    | Slot missing required prop                       |
| 47    | Lifetime parameterized component                 |
| 48    | Multiple components same prop names              |
| 49    | Children wrong return type                       |
| 50    | Children wrong type + missing prop               |
| 51    | Duplicate prop (caught at parse time)            |
| 52    | Duplicate optional prop (caught at parse time)   |
| 53    | Slot duplicate prop                              |
| 54    | Duplicate generic prop                           |
| 55    | Generic non-Fn bound wrong type                  |
| 56    | Generic Clone + Fn bound wrong type              |
| 57    | Children-only component, children missing        |
| 58    | Duplicate into prop                              |
| 59    | Two components, one error                        |
| 60    | User trait bound (correct)                       |
| 61    | User trait bound (wrong type)                    |

### Compiler Assumption Tests (`leptos_macro/tests/compiler_assumptions/`)

These pin undocumented rustc behaviors the two-step pre-check relies on. If any fail after
a nightly update, the error localization strategy may need revision.

| Test | Assumption                                                         |
|------|--------------------------------------------------------------------|
| 01   | UFCS does NOT produce `{error}` type (both E0277 and E0308 appear) |
| 02   | Method calls DO produce `{error}` type (only E0599, no E0308)      |
| 03   | E0599 does NOT show `on_unimplemented` for closures                |
| 04   | UFCS with concrete tuple pattern matching produces E0277           |
| 05   | `{error}` from method call is absorbed by subsequent UFCS call     |
| 06   | Ambiguous numeric literals produce E0689, not E0599                |

```bash
cargo +nightly test -p leptos_macro --test view
cargo +nightly test -p leptos_macro --test compiler_assumptions
TRYBUILD=overwrite cargo +nightly test -p leptos_macro --test view  # update snapshots
```

## Design Decisions

### Approaches that don't work for `{error}` propagation

- **UFCS alone**: Return type resolved by bidirectional inference — no `{error}`
- **UFCS + associated type**: Same — projection resolved by inference
- **`impl Trait` return**: Compiler resolves T through function signature
- **Local pass trait at call site**: Works (1 error!) but inaccessible from other modules
- **GAT return type**: Changes primary error from E0277 to E0599
- **Blanket `__CheckMissing` trait**: E0034 (multiple applicable items) with glob imports
- **`__Finish` UFCS wrapping `.build()`**: TypedBuilder needs specific type-state, can't be
  called inside a generic impl

### Why `#[diagnostic::do_not_recommend]` doesn't help

Each check trait has exactly one impl — there's no alternative to deprioritize. On bounded
`__Check_*` impls, it suppresses the clean E0277 `on_unimplemented` message. For closures,
it also suppresses Rust's native E0271/E0593 diagnostics.

### Tested hypotheses (rejected)

- **`label` attribute on `on_unimplemented`** (2026-02-18): Replaced the informative inline
  annotation (`the trait Fn() is not implemented for bool`) with a vague custom label. Rejected
  because the compiler default is more informative at the annotation point.
- **Removing `__check_missing()`** (2026-02-18): Exposed TypedBuilder's `.build()` errors —
  deprecation warnings and E0061 with internal type names like
  `InnerPropsBuilder_Error_Missing_required_field_foo`. Strictly worse output.

## Breaking Changes

- **`component_props_builder` removed**: The `view!` macro now uses
  `ComponentName::__builder()` instead of `component_props_builder(&ComponentName)`.
- **`component::*` restricted in prelude**: `leptos/src/lib.rs` exports specific items
  (`component_view`, `ComponentConstructor`, `Props`) instead of `component::*`.
