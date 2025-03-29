#![cfg(feature = "tokio")]

use any_spawner::Executor;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

#[tokio::test]
async fn test_executor_tick() {
    // Initialize the tokio executor
    Executor::init_tokio().expect("Failed to initialize tokio executor");

    let value = Arc::new(Mutex::new(false));
    let value_clone = value.clone();

    // Spawn a task that sets the value after a tick
    Executor::spawn(async move {
        Executor::tick().await;
        *value_clone.lock().unwrap() = true;
    });

    // Allow some time for the task to complete
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Check that the value was set
    assert!(*value.lock().unwrap());
}
