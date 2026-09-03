use super::{PartialPathMatch, PathSegment, PossibleRouteMatch};
use core::iter;
use std::borrow::Cow;

/// A segment that captures a value from the url and maps it to a key.
///
/// # Examples
/// ```rust
/// # (|| -> Option<()> { // Option does not impl Terminate, so no main
/// use leptos::prelude::*;
/// use leptos_router::{ParamSegment, PossibleRouteMatch, path};
///
/// let path = &"/hello";
///
/// // Manual definition
/// let manual = (ParamSegment("message"),);
/// let params = manual.test(path)?.params();
/// let (key, value) = params.last()?;
///
/// assert_eq!(key, "message");
/// assert_eq!(value, "hello");
///
/// // Macro definition
/// let using_macro = path!("/:message");
/// let params = using_macro.test(path)?.params();
/// let (key, value) = params.last()?;
///
/// assert_eq!(key, "message");
/// assert_eq!(value, "hello");
///
/// # Some(())
/// # })().unwrap();
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ParamSegment(pub &'static str);

impl PossibleRouteMatch for ParamSegment {
    fn optional(&self) -> bool {
        false
    }

    fn test<'a>(&self, path: &'a str) -> Option<PartialPathMatch<'a>> {
        let bytes = path.as_bytes();
        let has_leading_slash = bytes.first() == Some(&b'/');
        if !has_leading_slash && !path.is_empty() {
            return None;
        }
        let offset = usize::from(has_leading_slash);
        // the param ends at the next `/`; `/` is ASCII, so scanning bytes is
        // equivalent to scanning chars without the UTF-8 decoding
        let param_len = bytes[offset..]
            .iter()
            .position(|&byte| byte == b'/')
            .unwrap_or(bytes.len() - offset);
        let matched_len = offset + param_len;

        if matched_len == 0 || (matched_len == 1 && has_leading_slash) {
            return None;
        }

        debug_assert!(path.is_char_boundary(matched_len));
        let (matched, remaining) = path.split_at(matched_len);
        let param_value = vec![(
            Cow::Borrowed(self.0),
            path[offset..param_len + offset].to_string(),
        )];
        Some(PartialPathMatch::new(remaining, param_value, matched))
    }

    fn generate_path(&self, path: &mut Vec<PathSegment>) {
        path.push(PathSegment::Param(self.0.into()));
    }
}

/// A segment that captures all remaining values from the url and maps it to a key.
///
/// A [`WildcardSegment`] __must__ be the last segment of your path definition.
///
/// ```rust
/// # (|| -> Option<()> { // Option does not impl Terminate, so no main
/// use leptos::prelude::*;
/// use leptos_router::{
///     ParamSegment, PossibleRouteMatch, StaticSegment, WildcardSegment, path,
/// };
///
/// let path = &"/echo/send/sync/and/static";
///
/// // Manual definition
/// let manual = (StaticSegment("echo"), WildcardSegment("kitchen_sink"));
/// let params = manual.test(path)?.params();
/// let (key, value) = params.last()?;
///
/// assert_eq!(key, "kitchen_sink");
/// assert_eq!(value, "send/sync/and/static");
///
/// // Macro definition
/// let using_macro = path!("/echo/*else");
/// let params = using_macro.test(path)?.params();
/// let (key, value) = params.last()?;
///
/// assert_eq!(key, "else");
/// assert_eq!(value, "send/sync/and/static");
///
/// // This fails to compile because the macro will catch the bad ordering
/// // let bad = path!("/echo/*foo/bar/:baz");
///
/// // This compiles but may not work as you expect at runtime.
/// (
///     StaticSegment("echo"),
///     WildcardSegment("foo"),
///     ParamSegment("baz"),
/// );
///
/// # Some(())
/// # })().unwrap();
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct WildcardSegment(pub &'static str);

impl PossibleRouteMatch for WildcardSegment {
    fn optional(&self) -> bool {
        false
    }

    fn test<'a>(&self, path: &'a str) -> Option<PartialPathMatch<'a>> {
        let bytes = path.as_bytes();
        let has_leading_slash = bytes.first() == Some(&b'/');
        if !has_leading_slash && !path.is_empty() {
            return None;
        }
        let offset = usize::from(has_leading_slash);
        // the wildcard consumes the entire remaining path, so no scan is
        // needed at all
        let param_len = bytes.len() - offset;
        let matched_len = offset + param_len;

        debug_assert!(path.is_char_boundary(matched_len));
        let (matched, remaining) = path.split_at(matched_len);
        let param_value = iter::once((
            Cow::Borrowed(self.0),
            path[offset..param_len + offset].to_string(),
        ));
        Some(PartialPathMatch::new(
            remaining,
            param_value.into_iter().collect(),
            matched,
        ))
    }

    fn generate_path(&self, path: &mut Vec<PathSegment>) {
        path.push(PathSegment::Splat(self.0.into()));
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct OptionalParamSegment(pub &'static str);

impl PossibleRouteMatch for OptionalParamSegment {
    fn optional(&self) -> bool {
        true
    }

    fn test<'a>(&self, path: &'a str) -> Option<PartialPathMatch<'a>> {
        let bytes = path.as_bytes();
        let has_leading_slash = bytes.first() == Some(&b'/');
        if !has_leading_slash && !path.is_empty() {
            return Some(PartialPathMatch::new(path, Vec::new(), ""));
        }
        let offset = usize::from(has_leading_slash);
        // the param ends at the next `/`; `/` is ASCII, so scanning bytes is
        // equivalent to scanning chars without the UTF-8 decoding
        let param_len = bytes[offset..]
            .iter()
            .position(|&byte| byte == b'/')
            .unwrap_or(bytes.len() - offset);
        let matched_len = offset + param_len;

        let matched_len = if matched_len == 1 && has_leading_slash {
            0
        } else {
            matched_len
        };
        debug_assert!(path.is_char_boundary(matched_len));
        let (matched, remaining) = path.split_at(matched_len);
        let param_value = (matched_len > 0)
            .then(|| {
                (
                    Cow::Borrowed(self.0),
                    path[offset..param_len + offset].to_string(),
                )
            })
            .into_iter()
            .collect();
        Some(PartialPathMatch::new(remaining, param_value, matched))
    }

    fn generate_path(&self, path: &mut Vec<PathSegment>) {
        path.push(PathSegment::OptionalParam(self.0.into()));
    }
}

#[cfg(test)]
mod tests {
    use super::PossibleRouteMatch;
    use crate::{
        OptionalParamSegment, ParamSegment, StaticSegment, WildcardSegment,
    };

    #[test]
    fn single_param_match() {
        let path = "/foo";
        let def = ParamSegment("a");
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo");
        assert_eq!(matched.remaining(), "");
        let params = matched.params();
        assert_eq!(params[0], ("a".into(), "foo".into()));
    }

    #[test]
    fn single_param_match_with_trailing_slash() {
        let path = "/foo/";
        let def = ParamSegment("a");
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo");
        assert_eq!(matched.remaining(), "/");
        let params = matched.params();
        assert_eq!(params[0], ("a".into(), "foo".into()));
    }

    #[test]
    fn tuple_of_param_matches() {
        let path = "/foo/bar";
        let def = (ParamSegment("a"), ParamSegment("b"));
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo/bar");
        assert_eq!(matched.remaining(), "");
        let params = matched.params();
        assert_eq!(params[0], ("a".into(), "foo".into()));
        assert_eq!(params[1], ("b".into(), "bar".into()));
    }

    #[test]
    fn multi_byte_param_match() {
        let path = "/日本語/x";
        let def = ParamSegment("a");
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/日本語");
        assert_eq!(matched.remaining(), "/x");
        let params = matched.params();
        assert_eq!(params[0], ("a".into(), "日本語".into()));
    }

    #[test]
    fn multi_byte_wildcard_match() {
        let path = "/日本語/x";
        let def = WildcardSegment("rest");
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/日本語/x");
        assert_eq!(matched.remaining(), "");
        let params = matched.params();
        assert_eq!(params[0], ("rest".into(), "日本語/x".into()));
    }

    #[test]
    fn multi_byte_optional_param_match() {
        let path = "/héllo";
        let def = OptionalParamSegment("a");
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/héllo");
        assert_eq!(matched.remaining(), "");
        let params = matched.params();
        assert_eq!(params[0], ("a".into(), "héllo".into()));
    }

    #[test]
    fn non_slash_multi_byte_param_does_not_match() {
        let def = ParamSegment("a");
        for path in ["éx", "日本語"] {
            assert!(def.test(path).is_none());
        }
    }

    #[test]
    fn non_slash_multi_byte_wildcard_does_not_match() {
        let def = WildcardSegment("rest");
        for path in ["éx", "日本語"] {
            assert!(def.test(path).is_none());
        }
    }

    #[test]
    fn non_slash_multi_byte_optional_param_matches_nothing() {
        let def = OptionalParamSegment("a");
        for path in ["éx", "日本語"] {
            let matched = def.test(path).expect("couldn't match route");
            assert_eq!(matched.matched(), "");
            assert_eq!(matched.remaining(), path);
            assert!(matched.params().is_empty());
        }
    }

    #[test]
    fn non_slash_ascii_param_segments_do_not_match() {
        assert!(ParamSegment("a").test("abc").is_none());
        assert!(WildcardSegment("rest").test("abc").is_none());
    }

    #[test]
    fn leading_slash_multi_byte_segments_match_fully() {
        let path = "/é";
        for matched in [
            ParamSegment("a").test(path).expect("couldn't match route"),
            WildcardSegment("rest")
                .test(path)
                .expect("couldn't match route"),
            OptionalParamSegment("a")
                .test(path)
                .expect("couldn't match route"),
        ] {
            assert_eq!(matched.matched(), path);
            assert_eq!(matched.remaining(), "");
        }
    }

    #[test]
    fn splat_should_match_all() {
        let path = "/foo/bar/////";
        let def = (
            StaticSegment("foo"),
            StaticSegment("bar"),
            WildcardSegment("rest"),
        );
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo/bar/////");
        assert_eq!(matched.remaining(), "");
        let params = matched.params();
        assert_eq!(params[0], ("rest".into(), "////".into()));
    }

    #[test]
    fn optional_param_can_match() {
        let path = "/foo";
        let def = OptionalParamSegment("a");
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo");
        assert_eq!(matched.remaining(), "");
        let params = matched.params();
        assert_eq!(params[0], ("a".into(), "foo".into()));
    }

    #[test]
    fn optional_param_can_not_match() {
        let path = "/";
        let def = OptionalParamSegment("a");
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "");
        assert_eq!(matched.remaining(), "/");
        let params = matched.params();
        assert_eq!(params.first(), None);
    }

    #[test]
    fn optional_params_match_first() {
        let path = "/foo";
        let def = (OptionalParamSegment("a"), OptionalParamSegment("b"));
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo");
        assert_eq!(matched.remaining(), "");
        let params = matched.params();
        assert_eq!(params[0], ("a".into(), "foo".into()));
    }

    #[test]
    fn optional_params_can_match_both() {
        let path = "/foo/bar";
        let def = (OptionalParamSegment("a"), OptionalParamSegment("b"));
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo/bar");
        assert_eq!(matched.remaining(), "");
        let params = matched.params();
        assert_eq!(params[0], ("a".into(), "foo".into()));
        assert_eq!(params[1], ("b".into(), "bar".into()));
    }

    #[test]
    fn matching_after_optional_param() {
        let path = "/bar";
        let def = (OptionalParamSegment("a"), StaticSegment("bar"));
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/bar");
        assert_eq!(matched.remaining(), "");
        let params = matched.params();
        assert!(params.is_empty());
    }

    #[test]
    fn static_before_param() {
        let path = "/foo/bar";
        let def = (StaticSegment("foo"), ParamSegment("b"));
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo/bar");
        assert_eq!(matched.remaining(), "");
        let params = matched.params();
        assert_eq!(params[0], ("b".into(), "bar".into()));
    }

    #[test]
    fn static_before_optional_param() {
        let path = "/foo/bar";
        let def = (StaticSegment("foo"), OptionalParamSegment("b"));
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo/bar");
        assert_eq!(matched.remaining(), "");
        let params = matched.params();
        assert_eq!(params[0], ("b".into(), "bar".into()));
    }

    #[test]
    fn multiple_optional_params_match_first() {
        let path = "/foo/bar";
        let def = (
            OptionalParamSegment("a"),
            OptionalParamSegment("b"),
            StaticSegment("bar"),
        );
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo/bar");
        assert_eq!(matched.remaining(), "");
        let params = matched.params();
        assert_eq!(params[0], ("a".into(), "foo".into()));
    }

    #[test]
    fn multiple_optionals_can_match_both() {
        let path = "/foo/qux/bar";
        let def = (
            OptionalParamSegment("a"),
            OptionalParamSegment("b"),
            StaticSegment("bar"),
        );
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo/qux/bar");
        assert_eq!(matched.remaining(), "");
        let params = matched.params();
        assert_eq!(params[0], ("a".into(), "foo".into()));
        assert_eq!(params[1], ("b".into(), "qux".into()));
    }
}
