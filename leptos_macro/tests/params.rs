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
