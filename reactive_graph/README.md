An implementation of a fine-grained reactive system.

Fine-grained reactivity is an approach to modeling the flow of data through an interactive
application by composing together three categories of reactive primitives:

1. **Signals**: atomic units of state, which can be directly mutated.
2. **Computations**: derived values, which cannot be mutated directly but update whenever the signals
   they depend on change. These include both synchronous and asynchronous derived values.
3. **Effects**: side effects that synchronize the reactive system with the non-reactive world
   outside it.

Signals and computations are "source" nodes in the reactive graph, because an observer can
subscribe to them to respond to changes in their values. Effects and computations are "subscriber"
nodes, because they can listen to changes in other values.

```rust
use reactive_graph::{
    computed::ArcMemo,
    effect::Effect,
    prelude::{Read, Set},
    signal::ArcRwSignal,
};

let count = ArcRwSignal::new(1);
let double_count = ArcMemo::new({
    let count = count.clone();
    move |_| *count.read() * 2
});

// the effect will run once initially
Effect::new(move |_| {
    println!("double_count = {}", *double_count.read());
});

// updating `count` will propagate changes to the dependencies,
// causing the effect to run again
count.set(2);
```

This reactivity is called "fine grained" because updating the value of a signal only affects
the effects and computations that depend on its value, without requiring any diffing or update
calculations for other values.

This model is especially suitable for building user interfaces, i.e., long-lived systems in
which changes can begin from many different entry points. It is not particularly useful in
"run-once" programs like a CLI.

## Design Principles and Assumptions

- **Effects are expensive.** The library is built on the assumption that the side effects
  (making a network request, rendering something to the DOM, writing to disk) are orders of
  magnitude more expensive than propagating signal updates. As a result, the algorithm is
  designed to avoid re-running side effects unnecessarily, and is willing to sacrifice a small
  amount of raw update speed to that goal.
- **Automatic dependency tracking.** Dependencies are not specified as a compile-time list, but
  tracked at runtime. This in turn enables **dynamic dependency tracking**: subscribers
  unsubscribe from their sources between runs, which means that a subscriber that contains a
  condition branch will not re-run when dependencies update that are only used in the inactive
  branch.
- **Asynchronous effect scheduling.** Effects are spawned as asynchronous tasks. This means
  that while updating a signal will immediately update its value, effects that depend on it
  will not run until the next "tick" of the async runtime. (This in turn means that the
  reactive system is _async runtime agnostic_: it can be used in the browser with
  `wasm-bindgen-futures`, in a native binary with `tokio`, in a GTK application with `glib`,
  etc.)

The reactive-graph algorithm used in this crate is based on that of
[Reactively](https://github.com/modderme123/reactively), as described
[in this article](https://dev.to/modderme123/super-charging-fine-grained-reactive-performance-47ph).
