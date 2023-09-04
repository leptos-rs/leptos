# Appendix: How does the Reactive System Work?

You don’t need to know very much about how the reactive system actually works in order to use the library successfully. But it’s always useful to understand what’s going on behind the scenes once you start working with the framework at an advanced level.

The reactive primitives you use are divided into three sets:

- **Signals** (`ReadSignal`/`WriteSignal`, `RwSignal`, `Resource`, `Trigger`) Values you can actively change to trigger reactive updates.
- **Computations** (`Memo`s) Values that depend on signals (or other computations) and derive a new reactive value through some pure computation.
- **Effects** Observers that listen to changes in some signals or computations and run a function, causing some side effect.

Derived signals are a kind of non-primitve computation: as plain closures, they simply allow you to refactor some repeated signal-based computation into a reusable function that can be called in multiple places, but they are not represented in the reactive system itself.

All the other primitives actually exist in the reactive system as nodes in a reactive graph.

Most of the work of the reactive system consists of propagating changes from signals to effects, possibly through some intervening memos.

The assumption of the reactive system is that effects (like rendering to the DOM or making a network request) are orders of magnitude more expensive than things like updating a Rust data structure inside your app.

So the **primary goal** of the reactive system is to **run effects as infrequently as possible**.

Leptos does this through the construction of a reactive graph.

> Leptos’s current reactive system is based heavily on the [Reactively](https://github.com/modderme123/reactively) library for JavaScript. You can read Milo’s article “[Super-Charging Fine-Grained Reactivity](https://dev.to/modderme123/super-charging-fine-grained-reactive-performance-47ph)” for an excellent account of its algorithm, as well as fine-grained reactivity in general—including some beautiful diagrams!

## The Reactive Graph

Signals, memos, and effects all share three characteristics:

- **Value** They have a current value: either the signal’s value, or (for memos and effects) the value returned by the previous run, if any.
- **Sources** Any other reactive primitives they depend on. (For signals, this is an empty set.)
- **Subscribers** Any other reactive primitives that depend on them. (For effects, this is an empty set.)

In reality then, signals, memos, and effects are just conventional names for one generic concept of a “node” in a reactive graph. Signals are always “root nodes,” with no sources/parents. Effects are always “leaf nodes,” with no subscribers. Memos typically have both sources and subscribers.

### Simple Dependencies

So imagine the following code:

```rust
// A
let (name, set_name) = create_signal("Alice");

// B
let name_upper = create_memo(move |_| name.with(|n| n.to_uppercase()));

// C
create_effect(move |_| {
	log!("{}", name_upper());
});

set_name("Bob");
```

You can easily imagine the reactive graph here: `name` is the only signal/origin node, the `create_effect` is the only effect/terminal node, and there’s one intervening memo.

```
A   (name)
|
B   (name_upper)
|
C   (the effect)
```

### Splitting Branches

Let’s make it a little more complex.

```rust
// A
let (name, set_name) = create_signal("Alice");

// B
let name_upper = create_memo(move |_| name.with(|n| n.to_uppercase()));

// C
let name_len = create_memo(move |_| name.len());

// D
create_effect(move |_| {
	log!("len = {}", name_len());
});

// E
create_effect(move |_| {
	log!("name = {}", name_upper());
});
```

This is also pretty straightforward: a signal source signal (`name`/`A`) divides into two parallel tracks: `name_upper`/`B` and `name_len`/`C`, each of which has an effect that depends on it.

```
 __A__
|     |
B     C
|     |
D     E
```

Now let’s update the signal.

```rust
set_name("Bob");
```

We immediately log

```
len = 3
name = BOB
```

Let’s do it again.

```rust
set_name("Tim");
```

The log should shows

```
name = TIM
```

`len = 3` does not log again.

Remember: the goal of the reactive system is to run effects as infrequently as possible. Changing `name` from `"Bob"` to `"Tim"` will cause each of the memos to re-run. But they will only notify their subscribers if their value has actually changed. `"BOB"` and `"TIM"` are different, so that effect runs again. But both names have the length `3`, so they do not run again.

### Reuniting Branches

One more example, of what’s sometimes called **the diamond problem**.

```rust
// A
let (name, set_name) = create_signal("Alice");

// B
let name_upper = create_memo(move |_| name.with(|n| n.to_uppercase()));

// C
let name_len = create_memo(move |_| name.len());

// D
create_effect(move |_| {
	log!("{} is {} characters long", name_upper(), name_len());
});
```

What does the graph look like for this?

```
 __A__
|     |
B     C
|     |
|__D__|
```

You can see why it's called the “diamond problem.” If I’d connected the nodes with straight lines instead of bad ASCII art, it would form a diamond: two memos, each of which depend on a signal, which feed into the same effect.

A naive, push-based reactive implementation would cause this effect to run twice, which would be bad. (Remember, our goal is to run effects as infrequently as we can.) For example, you could implement a reactive system such that signals and memos immediately propagate their changes all the way down the graph, through each dependency, essentially traversing the graph depth-first. In other words, updating `A` would notify `B`, which would notify `D`; then `A` would notify `C`, which would notify `D` again. This is both inefficient (`D` runs twice) and glitchy (`D` actually runs with the incorrect value for the second memo during its first run.)

## Solving the Diamond Problem

Any reactive implementation worth its salt is dedicated to solving this issue. There are a number of different approaches (again, [see Milo’s article](https://dev.to/modderme123/super-charging-fine-grained-reactive-performance-47ph) for an excellent overview).

Here’s how ours works, in brief.

A reactive node is always in one of three states:

- `Clean`: it is known not to have changed
- `Check`: it is possible it has changed
- `Dirty`: it has definitely changed

Updating a signal `Dirty` marks that signal `Dirty`, and marks all its descendants `Check`, recursively. Any of its descendants that are effects are added to a queue to be re-run.

```
    ____A (DIRTY)___
   |               |
B (CHECK)    C (CHECK)
   |               |
   |____D (CHECK)__|
```

Now those effects are run. (All of the effects will be marked `Check` at this point.) Before re-running its computation, the effect checks its parents to see if they are dirty. So

- So `D` goes to `B` and checks if it is `Dirty`.
- But `B` is also marked `Check`. So `B` does the same thing:
  - `B` goes to `A`, and finds that it is `Dirty`.
  - This means `B` needs to re-run, because one of its sources has changed.
  - `B` re-runs, generating a new value, and marks itself `Clean`
  - Because `B` is a memo, it then checks its prior value against the new value.
  - If they are the same, `B` returns "no change." Otherwise, it returns "yes, I changed."
- If `B` returned “yes, I changed,” `D` knows that it definitely needs to run and re-runs immediately before checking any other sources.
- If `B` returned “no, I didn’t change,” `D` continues on to check `C` (see process above for `B`.)
- If neither `B` nor `C` has changed, the effect does not need to re-run.
- If either `B` or `C` did change, the effect now re-runs.

Because the effect is only marked `Check` once and only queued once, it only runs once.

If the naive version was a “push-based” reactive system, simply pushing reactive changes all the way down the graph and therefore running the effect twice, this version could be called “push-pull.” It pushes the `Check` status all the way down the graph, but then “pulls” its way back up. In fact, for large graphs it may end up bouncing back up and down and left and right on the graph as it tries to determine exactly which nodes need to re-run.

**Note this important trade-off**: Push-based reactivity propagates signal changes more quickly, at the expense of over-re-running memos and effects. Remember: the reactive system is designed to minimize how often you re-run effects, on the (accurate) assumption that side effects are orders of magnitude more expensive than this kind of cache-friendly graph traversal happening entirely inside the library’s Rust code. The measurement of a good reactive system is not how quickly it propagates changes, but how quickly it propagates changes _without over-notifying_.

## Memos vs. Signals

Note that signals always notify their children; i.e., a signal is always marked `Dirty` when it updates, even if its new value is the same as the old value. Otherwise, we’d have to require `PartialEq` on signals, and this is actually quite an expensive check on some types. (For example, add an unnecessary equality check to something like `some_vec_signal.update(|n| n.pop())` when it’s clear that it has in fact changed.)

Memos, on the other hand, check whether they change before notifying their children. They only run their calculation once, no matter how many times you `.get()` the result, but they run whenever their signal sources change. This means that if the memo’s computation is _very_ expensive, you may actually want to memoize its inputs as well, so that the memo only re-calculates when it is sure its inputs have changed.

## Memos vs. Derived Signals

All of this is cool, and memos are pretty great. But most actual applications have reactive graphs that are quite shallow and quite wide: you might have 100 source signals and 500 effects, but no memos or, in rare case, three or four memos between the signal and the effect. Memos are extremely good at what they do: limiting how often they notify their subscribers that they have changed. But as this description of the reactive system should show, they come with overhead in two forms:

1. A `PartialEq` check, which may or may not be expensive.
2. Added memory cost of storing another node in the reactive system.
3. Added computational cost of reactive graph traversal.

In cases in which the computation itself is cheaper than this reactive work, you should avoid “over-wrapping” with memos and simply use derived signals. Here’s a great example in which you should never use a memo:

```rust
let (a, set_a) = create_signal(1);
// none of these make sense as memos
let b = move || a() + 2;
let c = move || b() % 2 == 0;
let d = move || if c() { "even" } else { "odd" };

set_a(2);
set_a(3);
set_a(5);
```

Even though memoizing would technically save an extra calculation of `d` between setting `a` to `3` and `5`, these calculations are themselves cheaper than the reactive algorithm.

At the very most, you might consider memoizing the final node before running some expensive side effect:

```rust
let text = create_memo(move |_| {
    d()
});
create_effect(move |_| {
    engrave_text_into_bar_of_gold(&text());
});
```
