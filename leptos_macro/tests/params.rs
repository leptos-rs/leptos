use leptos::prelude::*;
use leptos_router::params::Params;

#[derive(PartialEq, Debug, Params)]
struct UserInfo {
    user_id: Option<String>,
    email: Option<String>,
    r#type: Option<i32>,
    not_found: Option<i32>,
}

#[test]
fn params_test() {
    let mut map = leptos_router::params::ParamsMap::new();
    map.insert("user_id", "12".to_owned());
    map.insert("email", "em@il".to_owned());
    map.insert("type", "12".to_owned());
    let user_info = UserInfo::from_map(&map).unwrap();
    assert_eq!(
        UserInfo {
            email: Some("em@il".to_owned()),
            user_id: Some("12".to_owned()),
            r#type: Some(12),
            not_found: None,
        },
        user_info
    );
}

#[test]
fn params_to_map_test() {
    let user_info = UserInfo {
        user_id: Some("1337".to_owned()),
        email: Some("em@il".to_owned()),
        r#type: Some(12),
        not_found: None,
    };
    let map = user_info.to_map().unwrap();

    assert_eq!(Some("1337"), map.get_str("user_id"));
    assert_eq!(Some("em@il"), map.get_str("email"));
    assert_eq!(Some("12"), map.get_str("type"));
    assert_eq!(None, map.get_str("not_found"));
}

#[test]
fn params_to_map_test_no_options() {
    let my_params = MyParams {
        foo: -12,
        bar: 2,
        baz: String::from("Hello world!"),
    };
    let map = my_params.to_map().unwrap();

    assert_eq!(Some("-12"), map.get_str("foo"));
    assert_eq!(Some("2"), map.get_str("bar"));
    assert_eq!(Some("Hello world!"), map.get_str("baz"));
}

#[test]
fn params_from_map_test_no_options() {
    let mut map = leptos_router::params::ParamsMap::new();
    map.insert("foo", "-12".to_owned());
    map.insert("bar", "2".to_owned());
    map.insert("baz", "Hello world!".to_owned());
    let my_params = MyParams::from_map(&map).unwrap();
    assert_eq!(
        MyParams {
            foo: -12,
            bar: 2,
            baz: String::from("Hello world!")
        },
        my_params
    );
}

#[derive(PartialEq, Debug, Params)]
struct MyParams {
    foo: i32,
    bar: usize,
    baz: String,
}
