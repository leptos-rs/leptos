#[test]
fn context() {
    use leptos_reactive::{
        create_isomorphic_effect, create_runtime, provide_context, use_context,
    };

    let runtime = create_runtime();

    create_isomorphic_effect({
        move |_| {
            provide_context(String::from("test"));
            assert_eq!(use_context::<String>(), Some(String::from("test")));
            assert_eq!(use_context::<i32>(), None);
            assert_eq!(use_context::<bool>(), None);

            create_isomorphic_effect({
                move |_| {
                    provide_context(0i32);
                    assert_eq!(
                        use_context::<String>(),
                        Some(String::from("test"))
                    );
                    assert_eq!(use_context::<i32>(), Some(0));
                    assert_eq!(use_context::<bool>(), None);

                    create_isomorphic_effect({
                        move |_| {
                            provide_context(false);
                            assert_eq!(
                                use_context::<String>(),
                                Some(String::from("test"))
                            );
                            assert_eq!(use_context::<i32>(), Some(0));
                            assert_eq!(use_context::<bool>(), Some(false));
                        }
                    });
                }
            });
        }
    });

    runtime.dispose();
}
