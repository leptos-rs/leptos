#![cfg(feature = "futures-executor")]

use any_spawner::Executor;
use futures::channel::oneshot;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

#[test]
fn test_futures_executor() {
    // Initialize the futures executor
    Executor::init_futures_executor()
        .expect("Failed to initialize futures executor");

    let (tx, rx) = oneshot::channel();
    let result = Arc::new(Mutex::new(None));
    let result_clone = result.clone();

    // Spawn a task
    Executor::spawn(async move {
        tx.send(84).expect("Failed to send value");
    });

    // Spawn a task that waits for the result
    Executor::spawn(async move {
        match rx.await {
            Ok(val) => *result_clone.lock().unwrap() = Some(val),
            Err(_) => panic!("Failed to receive value"),
        }
    });

    // Poll a few times to ensure the task completes
    for _ in 0..10 {
        Executor::poll_local();
        std::thread::sleep(Duration::from_millis(10));

        if result.lock().unwrap().is_some() {
            break;
        }
    }

    assert_eq!(*result.lock().unwrap(), Some(84));
}
