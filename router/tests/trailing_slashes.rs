//! Some extra tests for Matcher NOT based on SolidJS's tests cases (as in matcher.rs)

use leptos_router::{params_map, Matcher};

#[test]
fn trailing_slashes_match_exactly() {
    let matcher = Matcher::new("/foo/");
    assert!(matches(&matcher, "/foo/"));
    assert!(!matches(&matcher, "/foo"));

    let matcher = Matcher::new("/foo/bar/");
    assert!(matches(&matcher, "/foo/bar/"));
    assert!(!matches(&matcher, "/foo/bar"));
    assert!(!matches(&matcher, "/foo/"));
    assert!(!matches(&matcher, "/foo"));
}

#[test]
fn trailng_slashes_params_match_exactly() {
    let matcher = Matcher::new("/foo/:bar/");
    assert!(matches(&matcher, "/foo/bar/"));
    assert!(matches(&matcher, "/foo/42/"));
    assert!(matches(&matcher, "/foo/%20/"));

    assert!(!matches(&matcher, "/foo/bar"));
    assert!(!matches(&matcher, "/foo/42"));
    assert!(!matches(&matcher, "/foo/%20"));

    let m = matcher.test("/foo/asdf/").unwrap();
    assert_eq!(m.params, params_map! { "bar" => "asdf" });
}

fn matches(m: &Matcher, loc: &str) -> bool {
    m.test(loc).is_some()
}
