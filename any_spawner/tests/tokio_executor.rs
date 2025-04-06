#![cfg(feature = "tokio")]

use any_spawner::Executor;
use futures::channel::oneshot;

#[tokio::test]
async fn test_tokio_executor() {
    // Initialize the tokio executor
    Executor::init_tokio().expect("Failed to initialize tokio executor");

    let (tx, rx) = oneshot::channel();

    // Spawn a task that sends a value
    Executor::spawn(async move {
        tx.send(42).expect("Failed to send value");
    });

    // Wait for the spawned task to complete
    assert_eq!(rx.await.unwrap(), 42);
}
