#[cfg(test)]
use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(not(feature = "ssr"))] {
        use leptos::{server, server_fn::Encoding, ServerFnError};

        #[test]
        fn server_default() {
            #[server]
            pub async fn my_server_action() -> Result<(), ServerFnError> {
                Ok(())
            }
            assert_eq!(MyServerAction::PREFIX, "/api");
            assert_eq!(&MyServerAction::URL[0..16], "my_server_action");
            assert_eq!(MyServerAction::ENCODING, Encoding::Url);
        }

        #[test]
        fn server_full_legacy() {
            #[server(FooBar, "/foo/bar", "Cbor", "my_path")]
            pub async fn my_server_action() -> Result<(), ServerFnError> {
                Ok(())
            }
            assert_eq!(FooBar::PREFIX, "/foo/bar");
            assert_eq!(FooBar::URL, "my_path");
            assert_eq!(FooBar::ENCODING, Encoding::Cbor);
        }

        #[test]
        fn server_all_keywords() {
            #[server(endpoint = "my_path", encoding = "Cbor", prefix = "/foo/bar", name = FooBar)]
            pub async fn my_server_action() -> Result<(), ServerFnError> {
                Ok(())
            }
            assert_eq!(FooBar::PREFIX, "/foo/bar");
            assert_eq!(FooBar::URL, "my_path");
            assert_eq!(FooBar::ENCODING, Encoding::Cbor);
        }

        #[test]
        fn server_mix() {
            #[server(FooBar, endpoint = "my_path")]
            pub async fn my_server_action() -> Result<(), ServerFnError> {
                Ok(())
            }
            assert_eq!(FooBar::PREFIX, "/api");
            assert_eq!(FooBar::URL, "my_path");
            assert_eq!(FooBar::ENCODING, Encoding::Url);
        }

        #[test]
        fn server_name() {
            #[server(name = FooBar)]
            pub async fn my_server_action() -> Result<(), ServerFnError> {
                Ok(())
            }
            assert_eq!(FooBar::PREFIX, "/api");
            assert_eq!(&FooBar::URL[0..16], "my_server_action");
            assert_eq!(FooBar::ENCODING, Encoding::Url);
        }

        #[test]
        fn server_prefix() {
            #[server(prefix = "/foo/bar")]
            pub async fn my_server_action() -> Result<(), ServerFnError> {
                Ok(())
            }
            assert_eq!(MyServerAction::PREFIX, "/foo/bar");
            assert_eq!(&MyServerAction::URL[0..16], "my_server_action");
            assert_eq!(MyServerAction::ENCODING, Encoding::Url);
        }

        #[test]
        fn server_encoding() {
            #[server(encoding = "GetJson")]
            pub async fn my_server_action() -> Result<(), ServerFnError> {
                Ok(())
            }
            assert_eq!(MyServerAction::PREFIX, "/api");
            assert_eq!(&MyServerAction::URL[0..16], "my_server_action");
            assert_eq!(MyServerAction::ENCODING, Encoding::GetJSON);
        }

        #[test]
        fn server_endpoint() {
            #[server(endpoint = "/path/to/my/endpoint")]
            pub async fn my_server_action() -> Result<(), ServerFnError> {
                Ok(())
            }
            assert_eq!(MyServerAction::PREFIX, "/api");
            assert_eq!(MyServerAction::URL, "/path/to/my/endpoint");
            assert_eq!(MyServerAction::ENCODING, Encoding::Url);
        }
    }
}
