// Test cases drawn from Solid Router
// see https://github.com/solidjs/solid-router/blob/main/test/utils.spec.ts

use leptos_router::resolve_path;

#[test]
fn resolve_path_should_normalize_base_arg() {
    assert_eq!(resolve_path("base", "", None), Some("/base".into()));
}

#[test]
fn resolve_path_should_normalize_path_arg() {
    assert_eq!(resolve_path("", "path", None), Some("/path".into()));
}

#[test]
fn resolve_path_should_normalize_from_arg() {
    assert_eq!(resolve_path("", "", Some("from")), Some("/from".into()));
}

#[test]
fn resolve_path_should_return_default_when_all_empty() {
    assert_eq!(resolve_path("", "", None), Some("/".into()));
}

#[test]
fn resolve_path_should_resolve_root_against_base_and_ignore_from() {
    assert_eq!(
        resolve_path("/base", "/", Some("/base/foo")),
        Some("/base".into())
    );
}

#[test]
fn resolve_path_should_resolve_rooted_paths_against_base_and_ignore_from() {
    assert_eq!(
        resolve_path("/base", "/bar", Some("/base/foo")),
        Some("/base/bar".into())
    );
}

#[test]
fn resolve_path_should_resolve_empty_path_against_from() {
    assert_eq!(
        resolve_path("/base", "", Some("/base/foo")),
        Some("/base/foo".into())
    );
}

#[test]
fn resolve_path_should_resolve_relative_paths_against_from() {
    assert_eq!(
        resolve_path("/base", "bar", Some("/base/foo")),
        Some("/base/foo/bar".into())
    );
}

#[test]
fn resolve_path_should_prepend_base_if_from_doesnt_start_with_it() {
    assert_eq!(
        resolve_path("/base", "bar", Some("/foo")),
        Some("/base/foo/bar".into())
    );
}

#[test]
fn resolve_path_should_test_start_of_from_against_base_case_insensitive() {
    assert_eq!(
        resolve_path("/base", "bar", Some("BASE/foo")),
        Some("/BASE/foo/bar".into())
    );
}

#[test]
fn resolve_path_should_work_with_rooted_search_and_base() {
    assert_eq!(
        resolve_path("/base", "/?foo=bar", Some("/base/page")),
        Some("/base?foo=bar".into())
    );
}

#[test]
fn resolve_path_should_work_with_rooted_search() {
    assert_eq!(
        resolve_path("", "/?foo=bar", None),
        Some("/?foo=bar".into())
    );
}

#[test]
fn preserve_spaces() {
    assert_eq!(
        resolve_path(" foo ", " bar baz ", None),
        Some("/ foo / bar baz ".into())
    );
}

#[test]
fn will_resolve_if_path_has_scheme() {
    assert_eq!(
        resolve_path("", "http://example.com", None).as_deref(),
        Some("http://example.com")
    );
    assert_eq!(
        resolve_path("", "https://example.com", None).as_deref(),
        Some("https://example.com")
    );
    assert_eq!(
        resolve_path("", "example://google.com", None).as_deref(),
        Some("example://google.com")
    );
    assert_eq!(
        resolve_path("", "tel:+15555555555", None).as_deref(),
        Some("tel:+15555555555")
    );
    assert_eq!(
        resolve_path("", "mailto:name@example.com", None).as_deref(),
        Some("mailto:name@example.com")
    );
    assert_eq!(
        resolve_path("", "//relative-protocol", None).as_deref(),
        Some("//relative-protocol")
    );
}
