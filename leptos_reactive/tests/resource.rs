#[test]
fn resource_returns_last_future() {
    #[cfg(feature = "ssr")]
    {
        use futures::{channel::oneshot::channel, FutureExt};
        use leptos_reactive::{
            create_resource, create_runtime, create_signal,
            raw_scope_and_disposer, SignalGet, SignalSet,
        };
        use tokio::task;
        use tokio_test::block_on;

        block_on(task::LocalSet::new().run_until(async move {
            let (cx, disposer) = raw_scope_and_disposer(create_runtime());
            task::spawn_local(async move {
                // Set up a resource that can listen to two different futures that we can resolve independently
                let (tx_1, rx_1) = channel::<()>();
                let (tx_2, rx_2) = channel::<()>();
                let rx_1 = rx_1.shared();
                let rx_2 = rx_2.shared();

                let (channel_number, set_channel_number) = create_signal(cx, 1);

                let resource = create_resource(
                    cx,
                    move || channel_number.get(),
                    move |channel_number| {
                        let rx_1 = rx_1.clone();
                        let rx_2 = rx_2.clone();
                        async move {
                            match channel_number {
                                1 => rx_1.await,
                                2 => rx_2.await,
                                _ => unreachable!(),
                            }
                            .unwrap();

                            channel_number
                        }
                    },
                );

                // Switch to waiting to second future while first is still loading
                set_channel_number.set(2);

                // Resolve first future
                tx_1.send(()).unwrap();
                task::yield_now().await;

                // Resource should still be loading
                assert_eq!(resource.read(cx), None);

                // Resolve second future
                tx_2.send(()).unwrap();
                task::yield_now().await;

                // Resource should now be loaded
                assert_eq!(resource.read(cx), Some(2));
            })
            .await
            .unwrap();
            disposer.dispose();
        }));
    }
}
