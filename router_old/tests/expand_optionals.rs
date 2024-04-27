use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "ssr")] {
        use leptos_router::expand_optionals;

        #[test]
        fn expand_optionals_should_expand() {
            assert_eq!(expand_optionals("/foo/:x"), vec!["/foo/:x"]);
            assert_eq!(expand_optionals("/foo/:x?"), vec!["/foo", "/foo/:x"]);
            assert_eq!(expand_optionals("/bar/:x?/"), vec!["/bar/", "/bar/:x/"]);
            assert_eq!(
                expand_optionals("/foo/:x?/:y?/:z"),
                vec!["/foo/:z", "/foo/:x/:z", "/foo/:x/:y/:z"]
            );
            assert_eq!(
                expand_optionals("/foo/:x?/:y/:z?"),
                vec!["/foo/:y", "/foo/:x/:y", "/foo/:y/:z", "/foo/:x/:y/:z"]
            );
            assert_eq!(
                expand_optionals("/foo/:x?/bar/:y?/baz/:z?"),
                vec![
                    "/foo/bar/baz",
                    "/foo/:x/bar/baz",
                    "/foo/bar/:y/baz",
                    "/foo/:x/bar/:y/baz",
                    "/foo/bar/baz/:z",
                    "/foo/:x/bar/baz/:z",
                    "/foo/bar/:y/baz/:z",
                    "/foo/:x/bar/:y/baz/:z"
                ]
            )
        }
    }
}
