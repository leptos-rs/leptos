#[cfg(not(feature = "ssr"))]
pub mod tests {
    use leptos::{
        server,
        server_fn::{codec, Http, ServerFn, ServerFnError},
    };
    use std::any::TypeId;

    #[test]
    fn server_default() {
        #[server]
        pub async fn my_server_action() -> Result<(), ServerFnError> {
            Ok(())
        }
        assert_eq!(
            <MyServerAction as ServerFn>::PATH
                .trim_end_matches(char::is_numeric),
            "/api/my_server_action"
        );
        assert_eq!(
            TypeId::of::<<MyServerAction as ServerFn>::Protocol>(),
            TypeId::of::<Http<codec::PostUrl, codec::Json>>()
        );
    }

    #[test]
    fn server_full_legacy() {
        #[server(FooBar, "/foo/bar", "Cbor", "my_path")]
        pub async fn my_server_action() -> Result<(), ServerFnError> {
            Ok(())
        }
        assert_eq!(<FooBar as ServerFn>::PATH, "/foo/bar/my_path");
        assert_eq!(
            TypeId::of::<<FooBar as ServerFn>::Protocol>(),
            TypeId::of::<Http<codec::Cbor, codec::Cbor>>()
        );
    }

    #[test]
    fn server_all_keywords() {
        #[server(endpoint = "my_path", encoding = "Cbor", prefix = "/foo/bar", name = FooBar)]
        pub async fn my_server_action() -> Result<(), ServerFnError> {
            Ok(())
        }
        assert_eq!(<FooBar as ServerFn>::PATH, "/foo/bar/my_path");
        assert_eq!(
            TypeId::of::<<FooBar as ServerFn>::Protocol>(),
            TypeId::of::<Http<codec::Cbor, codec::Cbor>>()
        );
    }

    #[test]
    fn server_mix() {
        #[server(FooBar, endpoint = "my_path")]
        pub async fn my_server_action() -> Result<(), ServerFnError> {
            Ok(())
        }
        assert_eq!(<FooBar as ServerFn>::PATH, "/api/my_path");
        assert_eq!(
            TypeId::of::<<FooBar as ServerFn>::Protocol>(),
            TypeId::of::<Http<codec::PostUrl, codec::Json>>()
        );
    }

    #[test]
    fn server_name() {
        #[server(name = FooBar)]
        pub async fn my_server_action() -> Result<(), ServerFnError> {
            Ok(())
        }
        assert_eq!(
            <FooBar as ServerFn>::PATH.trim_end_matches(char::is_numeric),
            "/api/my_server_action"
        );
        assert_eq!(
            TypeId::of::<<FooBar as ServerFn>::Protocol>(),
            TypeId::of::<Http<codec::PostUrl, codec::Json>>()
        );
    }

    #[test]
    fn server_prefix() {
        #[server(prefix = "/foo/bar")]
        pub async fn my_server_action() -> Result<(), ServerFnError> {
            Ok(())
        }
        assert_eq!(
            <MyServerAction as ServerFn>::PATH
                .trim_end_matches(char::is_numeric),
            "/foo/bar/my_server_action"
        );
        assert_eq!(
            TypeId::of::<<MyServerAction as ServerFn>::Protocol>(),
            TypeId::of::<Http<codec::PostUrl, codec::Json>>()
        );
    }

    #[test]
    fn server_encoding() {
        #[server(encoding = "GetJson")]
        pub async fn my_server_action() -> Result<(), ServerFnError> {
            Ok(())
        }
        assert_eq!(
            <MyServerAction as ServerFn>::PATH
                .trim_end_matches(char::is_numeric),
            "/api/my_server_action"
        );
        assert_eq!(
            TypeId::of::<<MyServerAction as ServerFn>::Protocol>(),
            TypeId::of::<Http<codec::GetUrl, codec::Json>>()
        );
    }

    #[test]
    fn server_endpoint() {
        #[server(endpoint = "/path/to/my/endpoint")]
        pub async fn my_server_action() -> Result<(), ServerFnError> {
            Ok(())
        }
        assert_eq!(
            <MyServerAction as ServerFn>::PATH,
            "/api/path/to/my/endpoint"
        );
        assert_eq!(
            TypeId::of::<<MyServerAction as ServerFn>::Protocol>(),
            TypeId::of::<Http<codec::PostUrl, codec::Json>>()
        );
    }
}
