use cfg_if::cfg_if;

// Test cases drawn from Solid Router
// see https://github.com/solidjs/solid-router/blob/main/test/utils.spec.ts

cfg_if! {
    if #[cfg(feature = "ssr")] {
        use leptos_router::join_paths;

        #[test]
        fn join_paths_should_join_with_a_single_slash() {
            assert_eq!(join_paths("/foo", "bar"), "/foo/bar");
            assert_eq!(join_paths("/foo/", "bar"), "/foo/bar");
            assert_eq!(join_paths("/foo", "/bar"), "/foo/bar");
            assert_eq!(join_paths("/foo/", "/bar"), "/foo/bar");
        }

        #[test]
        fn join_paths_should_ensure_leading_slash() {
            assert_eq!(join_paths("/foo", ""), "/foo");
            assert_eq!(join_paths("foo", ""), "/foo");
            assert_eq!(join_paths("", "foo"), "/foo");
            assert_eq!(join_paths("", "/foo"), "/foo");
            assert_eq!(join_paths("/", "foo"), "/foo");
            assert_eq!(join_paths("/", "/foo"), "/foo");
        }

        #[test]
        fn join_paths_should_strip_tailing_slash_asterisk() {
            assert_eq!(join_paths("foo/*", ""), "/foo");
            assert_eq!(join_paths("foo/*", "/"), "/foo");
            assert_eq!(join_paths("/foo/*all", ""), "/foo");
            assert_eq!(join_paths("/foo/*", "bar"), "/foo/bar");
            assert_eq!(join_paths("/foo/*all", "bar"), "/foo/bar");
            assert_eq!(join_paths("/*", "foo"), "/foo");
            assert_eq!(join_paths("/*all", "foo"), "/foo");
            assert_eq!(join_paths("*", "foo"), "/foo");
        }

        #[test]
        fn join_paths_should_preserve_parameters() {
            assert_eq!(join_paths("/foo/:bar", ""), "/foo/:bar");
            assert_eq!(join_paths("/foo/:bar", "baz"), "/foo/:bar/baz");
            assert_eq!(join_paths("/foo", ":bar/baz"), "/foo/:bar/baz");
            assert_eq!(join_paths("", ":bar/baz"), "/:bar/baz");
        }
    }
}
