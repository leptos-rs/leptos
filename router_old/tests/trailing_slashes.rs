//! Some extra tests for Matcher NOT based on SolidJS's tests cases (as in matcher.rs)

use leptos_router::*;

#[test]
fn trailing_slashes_match_exactly() {
    let matcher = Matcher::new("/foo/");
    assert_matches(&matcher, "/foo/");
    assert_no_match(&matcher, "/foo");

    let matcher = Matcher::new("/foo/bar/");
    assert_matches(&matcher, "/foo/bar/");
    assert_no_match(&matcher, "/foo/bar");

    let matcher = Matcher::new("/");
    assert_matches(&matcher, "/");
    assert_matches(&matcher, "");

    let matcher = Matcher::new("");
    assert_matches(&matcher, "");

    // Despite returning a pattern of "", web servers (known: Actix-Web and Axum)
    // may send us a path of "/". We should match those at the root:
    assert_matches(&matcher, "/");
}

#[cfg(feature = "ssr")]
#[test]
fn trailing_slashes_params_match_exactly() {
    let matcher = Matcher::new("/foo/:bar/");
    assert_matches(&matcher, "/foo/bar/");
    assert_matches(&matcher, "/foo/42/");
    assert_matches(&matcher, "/foo/%20/");

    assert_no_match(&matcher, "/foo/bar");
    assert_no_match(&matcher, "/foo/42");
    assert_no_match(&matcher, "/foo/%20");

    let m = matcher.test("/foo/asdf/").unwrap();
    assert_eq!(m.params, params_map! { "bar" => "asdf" });
}

fn assert_matches(matcher: &Matcher, path: &str) {
    assert!(
        matches(matcher, path),
        "{matcher:?} should match path {path:?}"
    );
}

fn assert_no_match(matcher: &Matcher, path: &str) {
    assert!(
        !matches(matcher, path),
        "{matcher:?} should NOT match path {path:?}"
    );
}

fn matches(m: &Matcher, loc: &str) -> bool {
    m.test(loc).is_some()
}
