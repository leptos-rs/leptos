use leptos_router::{
    OptionalParamSegment, ParamSegment, StaticSegment, WildcardSegment,
};
use leptos_router_macro::path;

#[test]
fn parses_empty_string() {
    let output = path!("");
    assert!(output.eq(&()));
}

#[test]
fn parses_single_slash() {
    let output = path!("/");
    assert!(output.eq(&()));
}

#[test]
fn parses_single_asterisk() {
    let output = path!("*");
    assert!(output.eq(&()));
}

#[test]
fn parses_slash_asterisk() {
    let output = path!("/*");
    assert!(output.eq(&()));
}

#[test]
fn parses_asterisk_any() {
    let output = path!("/foo/:bar/*any");
    assert_eq!(
        output,
        (
            StaticSegment("foo"),
            ParamSegment("bar"),
            WildcardSegment("any")
        )
    );
}

#[test]
fn parses_hyphen() {
    let output = path!("/foo/bar-baz");
    assert_eq!(output, (StaticSegment("foo"), StaticSegment("bar-baz")));
}

#[test]
fn parses_rfc3976_unreserved() {
    let output = path!("/-._~");
    assert_eq!(output, (StaticSegment("-._~"),));
}

#[test]
fn parses_rfc3976_pchar_other() {
    let output = path!("/@");
    assert_eq!(output, (StaticSegment("@"),));
}

#[test]
fn parses_no_slashes() {
    let output = path!("home");
    assert_eq!(output, (StaticSegment("home"),));
}

#[test]
fn parses_no_leading_slash() {
    let output = path!("home");
    assert_eq!(output, (StaticSegment("home"),));
}

#[test]
fn parses_trailing_slash() {
    let output = path!("/home/");
    assert_eq!(output, (StaticSegment("home"), StaticSegment("/")));
}

#[test]
fn parses_single_static() {
    let output = path!("/home");
    assert_eq!(output, (StaticSegment("home"),));
}

#[test]
fn parses_single_param() {
    let output = path!("/:id");
    assert_eq!(output, (ParamSegment("id"),));
}

#[test]
fn parses_optional_param() {
    let output = path!("/:id?");
    assert_eq!(output, (OptionalParamSegment("id"),));
}

#[test]
fn parses_static_and_param() {
    let output = path!("/home/:id");
    assert_eq!(output, (StaticSegment("home"), ParamSegment("id"),));
}

#[test]
fn parses_mixed_segment_types() {
    let output = path!("/foo/:bar/*baz");
    assert_eq!(
        output,
        (
            StaticSegment("foo"),
            ParamSegment("bar"),
            WildcardSegment("baz")
        )
    );
}

#[test]
fn parses_trailing_slash_after_param() {
    let output = path!("/foo/:bar/");
    assert_eq!(
        output,
        (
            StaticSegment("foo"),
            ParamSegment("bar"),
            StaticSegment("/")
        )
    );
}

#[test]
fn parses_consecutive_static() {
    let output = path!("/foo/bar/baz");
    assert_eq!(
        output,
        (
            StaticSegment("foo"),
            StaticSegment("bar"),
            StaticSegment("baz")
        )
    );
}

#[test]
fn parses_consecutive_param() {
    let output = path!("/:foo/:bar/:baz");
    assert_eq!(
        output,
        (
            ParamSegment("foo"),
            ParamSegment("bar"),
            ParamSegment("baz")
        )
    );
}

#[test]
fn parses_consecutive_optional_param() {
    let output = path!("/:foo?/:bar?/:baz?");
    assert_eq!(
        output,
        (
            OptionalParamSegment("foo"),
            OptionalParamSegment("bar"),
            OptionalParamSegment("baz")
        )
    );
}

#[test]
fn parses_complex() {
    let output = path!("/home/:id/foo/:bar/:baz?/*any");
    assert_eq!(
        output,
        (
            StaticSegment("home"),
            ParamSegment("id"),
            StaticSegment("foo"),
            ParamSegment("bar"),
            OptionalParamSegment("baz"),
            WildcardSegment("any"),
        )
    );
}

// #[test]
// fn deny_consecutive_slashes() {
//     let _ = path!("/////foo///bar/////baz/");
// }
//
// #[test]
// fn deny_invalid_segment() {
//     let _ = path!("/foo/^/");
// }
//
// #[test]
// fn deny_non_trailing_wildcard_segment() {
//     let _ = path!("/home/*any/end");
// }
//
// #[test]
// fn deny_invalid_wildcard() {
//     let _ = path!("/home/any*");
// }
