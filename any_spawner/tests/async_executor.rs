#![cfg(feature = "async-executor")]

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

// A simple async executor for testing
struct TestExecutor {
    tasks: Mutex<Vec<Pin<Box<dyn Future<Output = ()> + Send + 'static>>>>,
}

impl TestExecutor {
    fn new() -> Self {
        TestExecutor {
            tasks: Mutex::new(Vec::new()),
        }
    }

    fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.tasks.lock().unwrap().push(Box::pin(future));
    }

    fn run_all(&self) {
        // Take all tasks out to process them
        let tasks = self.tasks.lock().unwrap().drain(..).collect::<Vec<_>>();

        // Use a basic future executor to run each task to completion
        for mut task in tasks {
            // Use futures-lite's block_on to complete the future
            futures::executor::block_on(async {
                unsafe {
                    let task_mut = Pin::new_unchecked(&mut task);
                    let _ = std::future::Future::poll(
                        task_mut,
                        &mut std::task::Context::from_waker(
                            futures::task::noop_waker_ref(),
                        ),
                    );
                }
            });
        }
    }
}

#[test]
fn test_async_executor() {
    let executor = Arc::new(TestExecutor::new());
    let executor_clone = executor.clone();

    // Create a spawner function that will use our test executor
    let spawner = move |future| {
        executor_clone.spawn(future);
    };

    // Prepare test data
    let counter = Arc::new(Mutex::new(0));
    let counter_clone = counter.clone();

    // Use the spawner to spawn a task
    spawner(async move {
        *counter_clone.lock().unwrap() += 1;
    });

    // Run all tasks
    executor.run_all();

    // Check if the task completed correctly
    assert_eq!(*counter.lock().unwrap(), 1);
}
