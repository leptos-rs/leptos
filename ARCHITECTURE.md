# Architecture

The goal of this document is to make it easier for contributors (and anyone 
who’s interested!) to understand the architecture of the framework.

The whole Leptos framework is built from a series of layers. Each of these layers 
depends on the one below it, but each can be used independently from the ones 
built on top of it. While running a command like `cargo leptos new --git 
leptos-rs/start` pulls in the whole framework, it’s important to remember that 
none of this is magic: each layer of that onion can be stripped away and 
reimplemented, configured, or adapted as needed, incrementally.

## The Reactive System

The reactive system allows you to define dynamic values (signals), 
the relationships between them (derived signals and memos), and the side effects 
that run in response to them (effects).

These concepts are completely independent of the DOM and can be used to drive 
any kind of reactive updates. The reactive system is based on the assumption 
that data is relatively cheap, and side effects are relatively expensive. Its 
goal is to minimize those side effects (like updating the DOM or making a network 
requests) as infrequently as possible. 

The reactive system is implemented as a single data structure that exists at 
runtime. In exchange for giving ownership over a value to the reactive system 
(by creating a signal), you receive a `Copy + 'static` identifier for its 
location in the reactive system. This enables most of the ergonomics of storing
and sharing state, the use of callback closures without lifetime issues, etc.
This is implemented by storing signals in a slotmap arena. The signal, memo, 
and scope types that are exposed to users simply carry around an index into that 
slotmap.
