#[test]
fn context() {
    use leptos_reactive::{
        create_isomorphic_effect, create_root, create_runtime, create_scope,
        provide_context, use_context,
    };

    create_scope(create_runtime(), |cx| {
        create_root(cx, move |_| {
            create_isomorphic_effect({
                move |_| {
                    provide_context(cx, String::from("test"));
                    assert_eq!(
                        use_context::<String>(cx),
                        Some(String::from("test"))
                    );
                    assert_eq!(use_context::<i32>(cx), None);
                    assert_eq!(use_context::<bool>(cx), None);

                    create_isomorphic_effect({
                        move |_| {
                            provide_context(cx, 0i32);
                            assert_eq!(
                                use_context::<String>(cx),
                                Some(String::from("test"))
                            );
                            assert_eq!(use_context::<i32>(cx), Some(0));
                            assert_eq!(use_context::<bool>(cx), None);

                            create_isomorphic_effect({
                                move |_| {
                                    provide_context(cx, false);
                                    assert_eq!(
                                        use_context::<String>(cx),
                                        Some(String::from("test"))
                                    );
                                    assert_eq!(use_context::<i32>(cx), Some(0));
                                    assert_eq!(
                                        use_context::<bool>(cx),
                                        Some(false)
                                    );
                                }
                            });
                        }
                    });
                }
            });
        });
    })
    .dispose()
}
