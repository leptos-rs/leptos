use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "ssr")] {
        use leptos_router::{Url, params_map};

        macro_rules! assert_params_map {
            ([$($key:expr => $val:expr),*] , $actual:expr) => (
                assert_eq!(params_map!($($key => $val),*), $actual)
            );
        }

        #[test]
        fn test_param_with_plus_sign() {
            let url = Url::try_from("http://leptos.com?data=1%2B2%3D3").unwrap();
            assert_params_map!{
                ["data" => "1+2=3"],
                url.search_params
            };
        }

        #[test]
        fn test_param_with_ampersand() {
            let url = Url::try_from("http://leptos.com?data=true+%26+false+%3D+false").unwrap();
            assert_params_map!{
                ["data" => "true & false = false"],
                url.search_params
            };
        }

        #[test]
        fn test_complex_query_string() {
            let url = Url::try_from("http://leptos.com?data=Data%3A+%24+%26+%2B%2B+7").unwrap();
            assert_params_map!{
                ["data" => "Data: $ & ++ 7"],
                url.search_params
            };
        }

        #[test]
        fn test_multiple_query_params() {
            let url = Url::try_from("http://leptos.com?param1=value1&param2=value2").unwrap();
            assert_params_map!{
                [
                    "param1" => "value1",
                    "param2" => "value2"
                ],
                url.search_params
            };
        }
    }
}
