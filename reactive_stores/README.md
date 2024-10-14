# Stores

Stores are a data structure for nested reactivity.

The [`reactive_graph`](https://crates.io/crates/reactive_graph) crate provides primitives for fine-grained reactivity
via signals, memos, and effects.

This crate extends that reactivity to support reactive access to nested dested, without the need to create nested signals.

Using the `#[derive(Store)]` macro on a struct creates a series of getters that allow accessing each field. Individual fields 
can then be read as if they were signals. Changes to parents will notify their children, but changing one sibling field will  
not notify any of the others, nor will it require diffing those sibling fields (unlike earlier solutions using memoized “slices”).

This is published for use with the Leptos framework but can be used in any scenario where `reactive_graph` is being used 
for reactivity.
