Provides a simple trait that unwraps the locks provide by [`std::sync::RwLock`].

In every case, this is the same as calling `.expect("lock poisoned")`. However, it
does not use `.unwrap()` or `.expect()`, which makes it easier to distinguish from
other forms of unwrapping when reading code.

```rust
use or_poisoned::OrPoisoned;
use std::sync::RwLock;

let lock = RwLock::new(String::from("Hello!"));

let read = lock.read().or_poisoned();
// this is identical to
let read = lock.read().unwrap();
```
