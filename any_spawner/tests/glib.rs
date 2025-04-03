#![cfg(feature = "glib")]

use any_spawner::Executor;
use glib::{MainContext, MainLoop};
use serial_test::serial;
use std::{
    cell::Cell,
    future::Future,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

// Helper to run a future to completion on a dedicated glib MainContext.
// Returns true if the future completed within the timeout, false otherwise.
fn run_on_glib_context<F>(fut: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    let _ = Executor::init_glib();

    let context = MainContext::default();
    let main_loop = MainLoop::new(Some(&context), false);
    let main_loop_clone = main_loop.clone();

    Executor::spawn(async move {
        fut.await;
        main_loop_clone.quit();
    });

    main_loop.run();
}

// Helper to run a local (!Send) future on the glib context.
fn run_local_on_glib_context<F>(fut: F)
where
    F: Future<Output = ()> + 'static,
{
    let _ = Executor::init_glib();

    let context = MainContext::default();
    let main_loop = MainLoop::new(Some(&context), false);
    let main_loop_clone = main_loop.clone();

    Executor::spawn_local(async move {
        fut.await;
        main_loop_clone.quit();
    });

    main_loop.run();
}

// This test must run after a test that successfully initializes glib,
// or within its own process.
#[test]
#[serial]
fn test_glib_spawn() {
    let success_flag = Arc::new(AtomicBool::new(false));
    let flag_clone = success_flag.clone();

    run_on_glib_context(async move {
        // Simulate async work
        futures_lite::future::yield_now().await;
        flag_clone.store(true, Ordering::SeqCst);

        // We need to give the spawned task time to run.
        // The run_on_glib_context handles the main loop.
        // We just need to ensure spawn happened correctly.
        // Let's wait a tiny bit within the driving future to ensure spawn gets processed.
        glib::timeout_future(Duration::from_millis(10)).await;
    });

    assert!(
        success_flag.load(Ordering::SeqCst),
        "Spawned future did not complete successfully"
    );
}

// Similar conditions as test_glib_spawn regarding initialization state.
#[test]
#[serial]
fn test_glib_spawn_local() {
    let success_flag = Rc::new(Cell::new(false));
    let flag_clone = success_flag.clone();

    run_local_on_glib_context(async move {
        // Use Rc to make the future !Send
        let non_send_data = Rc::new(Cell::new(10));

        let data = non_send_data.get();
        assert_eq!(data, 10, "Rc data should be accessible");
        non_send_data.set(20); // Modify non-Send data

        // Simulate async work
        futures_lite::future::yield_now().await;

        assert_eq!(
            non_send_data.get(),
            20,
            "Rc data should persist modification"
        );
        flag_clone.set(true);

        // Wait a tiny bit
        glib::timeout_future(Duration::from_millis(10)).await;
    });

    assert!(
        success_flag.get(),
        "Spawned local future did not complete successfully"
    );
}

// Test Executor::tick with glib backend
#[test]
#[serial]
fn test_glib_tick() {
    run_on_glib_context(async {
        let value = Arc::new(Mutex::new(false));
        let value_clone = value.clone();

        // Spawn a task that sets the value after a tick
        Executor::spawn(async move {
            Executor::tick().await;
            *value_clone.lock().unwrap() = true;
        });

        // Allow some time for the task to complete
        glib::timeout_future(Duration::from_millis(10)).await;

        // Check that the value was set
        assert!(*value.lock().unwrap());
    });
}

// Test Executor::poll_local with glib backend (should be a no-op)
#[test]
#[serial]
fn test_glib_poll_local_is_no_op() {
    // Ensure glib executor is initialized
    let _ = Executor::init_glib();
    // poll_local for glib is configured as a no-op
    // Calling it should not panic or cause issues.
    Executor::poll_local();
    Executor::poll_local();

    println!("Executor::poll_local called successfully (expected no-op).");
}
