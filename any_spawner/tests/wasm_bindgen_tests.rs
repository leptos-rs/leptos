#![cfg(all(feature = "wasm-bindgen", target_family = "wasm"))]

use any_spawner::Executor;
use futures::channel::oneshot;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_wasm_bindgen_spawn_local() {
    // Initialize the wasm-bindgen executor
    let _ = Executor::init_wasm_bindgen();

    // Create a channel to verify the task completes
    let (tx, rx) = oneshot::channel();

    // Spawn a local task (wasm doesn't support sending futures between threads)
    Executor::spawn_local(async move {
        // Simulate some async work
        Executor::tick().await;
        tx.send(42).expect("Failed to send result");
    });

    // Wait for the task to complete
    let result = rx.await.expect("Failed to receive result");
    assert_eq!(result, 42);
}

#[wasm_bindgen_test]
async fn test_wasm_bindgen_tick() {
    // Initialize the wasm-bindgen executor if not already initialized
    let _ = Executor::init_wasm_bindgen();

    let flag = Arc::new(AtomicBool::new(false));
    let flag_clone = flag.clone();

    // Spawn a task that will set the flag
    Executor::spawn_local(async move {
        flag_clone.store(true, Ordering::SeqCst);
    });

    // Wait for a tick, which should allow the spawned task to run
    Executor::tick().await;

    // Verify the flag was set
    assert!(flag.load(Ordering::SeqCst));
}

#[wasm_bindgen_test]
async fn test_multiple_wasm_bindgen_tasks() {
    // Initialize once for all tests
    let _ = Executor::init_wasm_bindgen();

    // Create channels for multiple tasks
    let (tx1, rx1) = oneshot::channel();
    let (tx2, rx2) = oneshot::channel();

    // Spawn multiple tasks
    Executor::spawn_local(async move {
        tx1.send("task1").expect("Failed to send from task1");
    });

    Executor::spawn_local(async move {
        tx2.send("task2").expect("Failed to send from task2");
    });

    // Wait for both tasks to complete
    let (result1, result2) = futures::join!(rx1, rx2);

    assert_eq!(result1.unwrap(), "task1");
    assert_eq!(result2.unwrap(), "task2");
}

// This test verifies that spawn (not local) fails on wasm as expected
#[wasm_bindgen_test]
#[should_panic]
fn test_wasm_bindgen_spawn_errors() {
    let _ = Executor::init_wasm_bindgen();

    // Using should_panic to test that Executor::spawn panics in wasm
    Executor::spawn(async {
        // This should panic since wasm-bindgen doesn't support Send futures
    });
}
