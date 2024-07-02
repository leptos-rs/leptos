This crate makes it easier to write asynchronous code that is executor-agnostic, by providing a
utility that can be used to spawn tasks in a variety of executors.

It only supports single executor per program, but that executor can be set at runtime, anywhere
in your crate (or an application that depends on it).

This can be extended to support any executor or runtime that supports spawning [`Future`]s.

This is a least common denominator implementation in many ways. Limitations include:

- setting an executor is a one-time, global action
- no "join handle" or other result is returned from the spawn
- the `Future` must output `()`

```rust
use any_spawner::Executor;

Executor::init_futures_executor()
    .expect("executor should only be initialized once");

// spawn a thread-safe Future
Executor::spawn(async { /* ... */ });

// spawn a Future that is !Send
Executor::spawn_local(async { /* ... */ });
```
