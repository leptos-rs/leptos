# localized errors

UX improvement when working with the view! macro.

# Requirements

- Errors in the view macro must be reported where they occur.
- Errors should state in their first line what the root issue is.

# Assumptions

- Each `#[component]` component can produce a companion module or enum or typedef/struct, they can live alongside the
  function name.
- A companion modules MUST NOT hover rely on a `use super::*;` glob import to get access to all user-imported types, as
  `#[component]`s could be declared inside other functions (this is the case in doc tests!) and a module declared inside
  a function, even when using `use super::*;`, do not get access to types imported in the parent function.
- Therefore: A companion module, should it be generated, cannot use the components generics inside it, as they likely
  contain types defined by the user. However, behavioral-bound stripped generic could be used, as they won't use user
  defined types (verify this!!!).

## Findings

### UFCS vs Method Syntax for Component Pre-Checks

Empirically verified with rustc.

#### UFCS `<_ as Trait>::method(val)` when bound fails

| Scenario                                                   | Error code | `on_unimplemented` shown?                           | `{error}` propagation?          |
|------------------------------------------------------------|------------|-----------------------------------------------------|---------------------------------|
| Named type (e.g. `bool`, `String`)                         | E0277      | **Yes** — custom message is primary                 | **Yes** — downstream suppressed |
| Closure, wrong return type (`\|\| true` for `Fn() -> i32`) | E0271      | **No** — compiler gives its own "expected X, got Y" | **Yes** — downstream suppressed |
| Closure, wrong arity (`\|x\| 42` for `Fn() -> i32`)        | E0593      | **No** — compiler gives its own "takes N args"      | **Yes** — downstream suppressed |
| Non-Fn type as closure                                     | E0277      | **Yes**                                             | **Yes**                         |

#### Key insight: E0271/E0593 for closures are BETTER than custom messages

When a closure partially matches `Fn` (right trait family, wrong signature), the compiler produces targeted diagnostics:

- E0271: `"expected {closure} to return i32, but it returns bool"` — tells exact fix needed
- E0593: `"closure is expected to take 0 arguments, but it takes 1 argument"` — tells exact fix needed

These are **more actionable** than our generic `on_unimplemented` message (
`"{Self} is not a valid type for prop fun_b"`). The compiler's built-in closure diagnostics are superior for these
cases.

#### Method syntax `val.method()` when bound fails (current approach)

| Scenario               | Error code | `on_unimplemented` shown?                                                          | `{error}` propagation? |
|------------------------|------------|------------------------------------------------------------------------------------|------------------------|
| Named type (`bool`)    | E0599      | **Yes** — custom message is primary                                                | **Yes**                |
| Closure (any mismatch) | E0599      | **No** — ugly default: `"method __pass_foo exists but trait bounds not satisfied"` | **Yes**                |

#### Direct vs `__Check_*` intermediate trait

With `__Check_*` intermediate (`impl<T: Check> Pass for T`):

- Named types: `on_unimplemented` from `Pass` shows, **but** output includes TWO "required for" notes (one for `Check`,
  one for `Pass`) — noisier
- Closures: same E0271/E0593, but TWO "required for" notes

Without `__Check_*` (`impl<T: Fn() -> i32> Pass for T`):

- Named types: `on_unimplemented` shows, only ONE "required for" note — cleaner
- Closures: same E0271/E0593, only ONE note

**Conclusion: Direct bounds on `__Pass_*` (no `__Check_*`) produces cleaner output.**

#### Span behavior in UFCS

For `<_ as Trait>::method(val)`:

- Named types: error points to `_` token (the inferred Self type)
- Closures: error points to the value expression

In proc macros with `quote_spanned!{value_span=> <_ as Path>::method(val)}`, the `_` token gets the value span, so the
error localizes to the source value expression.

#### `{error}` propagation scope

- UFCS E0277/E0271/E0593 all produce `{error}` return type
- `{error}` propagates through **dependent** downstream expressions (same builder chain)
- Independent `let` statements are NOT suppressed by each other (correct behavior — each wrong prop gets its own error)

#### Summary: UFCS is strictly better than method syntax for pre-checks

| Criterion               | Method syntax (E0599)          | UFCS (E0277/E0271/E0593)         |
|-------------------------|--------------------------------|----------------------------------|
| Named types             | Nice `on_unimplemented`        | Nice `on_unimplemented`          |
| Closures (wrong return) | Ugly internal names            | Clear "expected X, got Y"        |
| Closures (wrong arity)  | Ugly internal names            | Clear "takes N args, expected M" |
| `{error}` propagation   | Yes                            | Yes                              |
| Trait complexity        | Needs `__Check_*` + `__Pass_*` | Just `__Pass_*`                  |
| Import requirements     | `use Trait as _;` needed       | None (UFCS is fully qualified)   |
