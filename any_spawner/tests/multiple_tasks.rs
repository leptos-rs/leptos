#![cfg(feature = "tokio")]

use any_spawner::Executor;
use futures::channel::oneshot;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn test_multiple_tasks() {
    Executor::init_tokio().expect("Failed to initialize tokio executor");

    let counter = Arc::new(Mutex::new(0));
    let tasks = 10;
    let mut handles = Vec::new();

    // Spawn multiple tasks that increment the counter
    for _ in 0..tasks {
        let counter_clone = counter.clone();
        let (tx, rx) = oneshot::channel();

        Executor::spawn(async move {
            *counter_clone.lock().unwrap() += 1;
            tx.send(()).expect("Failed to send completion signal");
        });

        handles.push(rx);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.expect("Task failed");
    }

    // Verify that all tasks incremented the counter
    assert_eq!(*counter.lock().unwrap(), tasks);
}
