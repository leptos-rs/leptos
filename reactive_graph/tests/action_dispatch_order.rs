//! Regression test for the out-of-order completion guard in `ArcAction`.
//!
//! Two dispatches can complete in the opposite order they were issued. The
//! action keeps a `version`/`value` pair guarded by a `dispatched` counter so
//! that a stale (older) dispatch cannot overwrite the value produced by a
//! newer one. The counter has to actually be incremented on every dispatch for
//! the guard to work.

use any_spawner::Executor;
use reactive_graph::{actions::ArcAction, owner::Owner, traits::GetUntracked};
use std::sync::Arc;
use tokio::sync::Notify;

/// Drive the executor until `cond` holds. The bound just turns a genuinely
/// stuck future into a loud failure instead of a hang.
async fn tick_until(mut cond: impl FnMut() -> bool) {
    for _ in 0..10_000 {
        if cond() {
            return;
        }
        Executor::tick().await;
    }
    panic!("condition was never satisfied within the tick budget");
}

#[tokio::test]
async fn older_dispatch_does_not_clobber_newer_result() {
    _ = Executor::init_tokio();
    let owner = Owner::new();
    owner.set();

    // The future for input `1` blocks on `gate`; the future for any other
    // input resolves immediately. This lets us force the *first* dispatch to
    // finish *after* the second one.
    let gate = Arc::new(Notify::new());
    let action = {
        let gate = gate.clone();
        ArcAction::<u32, u32>::new(move |n: &u32| {
            let n = *n;
            let gate = gate.clone();
            async move {
                if n == 1 {
                    gate.notified().await;
                }
                n
            }
        })
    };

    let value = action.value();
    let input = action.input();

    // Dispatch the "old" call; it parks on the gate. (`Notify` stores a permit
    // if `notify_one` is called before the waiter registers, so this future is
    // guaranteed to make progress once released regardless of poll ordering.)
    action.dispatch(1);

    // Dispatch the "new" call; it resolves immediately and commits `2`. Wait
    // for the value to actually be committed instead of assuming a fixed
    // number of ticks is enough to drive the spawned future.
    action.dispatch(2);
    tick_until(|| value.get_untracked() == Some(2)).await;
    assert_eq!(
        value.get_untracked(),
        Some(2),
        "newer dispatch should have committed its value"
    );

    // Now release the older call. With a working guard it must recognise that
    // a newer dispatch already completed and leave the value untouched.
    // `input` is reset to `None` only once the last in-flight dispatch has
    // resolved and its completion path (including the guarded value commit) has
    // run, so observing `None` is a synchronisation point strictly after any
    // clobber would have happened.
    gate.notify_one();
    tick_until(|| input.get_untracked().is_none()).await;

    assert_eq!(
        value.get_untracked(),
        Some(2),
        "stale dispatch must not overwrite the newer result"
    );
}
