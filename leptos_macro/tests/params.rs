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
