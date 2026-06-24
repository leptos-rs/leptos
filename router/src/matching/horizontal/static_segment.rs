use super::{PartialPathMatch, PathSegment, PossibleRouteMatch};
use std::fmt::Debug;

impl PossibleRouteMatch for () {
    fn optional(&self) -> bool {
        false
    }

    fn test<'a>(&self, path: &'a str) -> Option<PartialPathMatch<'a>> {
        Some(PartialPathMatch::new(path, vec![], ""))
    }

    fn generate_path(&self, _path: &mut Vec<PathSegment>) {}
}

pub trait AsPath {
    fn as_path(&self) -> &'static str;
}

impl AsPath for &'static str {
    fn as_path(&self) -> &'static str {
        self
    }
}

/// A segment that is expected to be static. Not requiring mapping into params.
///
/// Should work exactly as you would expect.
///
/// # Examples
/// ```rust
/// # (|| -> Option<()> { // Option does not impl Terminate, so no main
/// use leptos::prelude::*;
/// use leptos_router::{PossibleRouteMatch, StaticSegment, path};
///
/// let path = &"/users";
///
/// // Manual definition
/// let manual = (StaticSegment("users"),);
/// let matched = manual.test(path)?;
/// assert_eq!(matched.matched(), "/users");
///
/// // Params are empty as we had no `ParamSegement`s or `WildcardSegment`s
/// // If you did have additional dynamic segments, this would not be empty.
/// assert_eq!(matched.params().len(), 0);
///
/// // Macro definition
/// let using_macro = path!("/users");
/// let matched = manual.test(path)?;
/// assert_eq!(matched.matched(), "/users");
///
/// assert_eq!(matched.params().len(), 0);
///
/// # Some(())
/// # })().unwrap();
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct StaticSegment<T: AsPath>(pub T);

impl<T: AsPath> PossibleRouteMatch for StaticSegment<T> {
    fn optional(&self) -> bool {
        false
    }

    fn test<'a>(&self, path: &'a str) -> Option<PartialPathMatch<'a>> {
        let segment = self.0.as_path();
        let seg = segment.as_bytes();
        let bytes = path.as_bytes();
        let mut matched_len = 0;
        let mut path_idx = 0;
        let mut seg_idx = 0;
        let mut has_matched = segment.is_empty() || segment == "/";

        // match an initial /
        if bytes.first() == Some(&b'/') {
            path_idx += 1;

            if !segment.is_empty() {
                matched_len += 1;
            }
            if seg.first() == Some(&b'/') || segment.is_empty() {
                seg_idx += 1;
            }
        } else if !path.is_empty() {
            // Path must start with `/` otherwise we are not certain about being at the beginning of the segment in the path
            return None;
        }

        // compare the path against the segment byte by byte: every comparison
        // is an exact-equality check, and matching only stops at the ASCII `/`
        // or at the end of the path or segment (both valid UTF-8), so
        // `matched_len` always lands on a character boundary
        while path_idx < bytes.len() {
            let byte = bytes[path_idx];
            path_idx += 1;
            let expected = seg.get(seg_idx).copied();
            seg_idx += 1;
            // when we get a closing /, stop matching
            if byte == b'/' {
                if expected.is_some() {
                    return None;
                }
                break;
            } else if expected.is_none() {
                break;
            }
            // if the next byte in the path matches the
            // next byte in the segment, add it to the match
            else if Some(byte) == expected {
                has_matched = true;
                matched_len += 1;
            }
            // otherwise, this route doesn't match and we should
            // return None
            else {
                return None;
            }
        }

        // if we still have remaining, unmatched bytes in this segment, it was not a match
        if seg_idx < seg.len() {
            return None;
        }

        // build the match object
        let (matched, remaining) = if matched_len == 1 && path.starts_with('/')
        {
            // If only thing that matched is `/` we can't eat it, otherwise next invocation of the
            // test function will not be able to tell that we are matching from the beginning of the path segment
            ("/", path)
        } else {
            // the remaining is built from the path in, with the slice moved
            // by the length of this match
            path.split_at(matched_len)
        };
        has_matched.then(|| PartialPathMatch::new(remaining, vec![], matched))
    }

    fn generate_path(&self, path: &mut Vec<PathSegment>) {
        path.push(PathSegment::Static(self.0.as_path().into()))
    }
}

#[cfg(test)]
mod tests {
    use super::{PossibleRouteMatch, StaticSegment};
    use crate::AsPath;

    #[derive(Debug, Clone)]
    enum Paths {
        Foo,
        Bar,
    }

    impl AsPath for Paths {
        fn as_path(&self) -> &'static str {
            match self {
                Foo => "foo",
                Bar => "bar",
            }
        }
    }

    use Paths::*;

    #[test]
    fn single_static_match() {
        let path = "/foo";
        let def = StaticSegment("foo");
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo");
        assert_eq!(matched.remaining(), "");
        let params = matched.params();
        assert!(params.is_empty());
    }

    #[test]
    fn single_static_match_on_enum() {
        let path = "/foo";
        let def = StaticSegment(Foo);
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo");
        assert_eq!(matched.remaining(), "");
        let params = matched.params();
        assert!(params.is_empty());
    }

    #[test]
    fn single_static_mismatch() {
        let path = "/foo";
        let def = StaticSegment("bar");
        assert!(def.test(path).is_none());
    }

    #[test]
    fn single_static_mismatch_on_enum() {
        let path = "/foo";
        let def = StaticSegment(Bar);
        assert!(def.test(path).is_none());
    }

    #[test]
    fn single_static_match_with_trailing_slash() {
        let path = "/foo/";
        let def = StaticSegment("foo");
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo");
        assert_eq!(matched.remaining(), "/");
        let params = matched.params();
        assert!(params.is_empty());
    }

    #[test]
    fn single_static_match_with_trailing_slash_on_enum() {
        let path = "/foo/";
        let def = StaticSegment(Foo);
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo");
        assert_eq!(matched.remaining(), "/");
        let params = matched.params();
        assert!(params.is_empty());
    }

    #[test]
    fn tuple_of_static_matches() {
        let path = "/foo/bar";
        let def = (StaticSegment("foo"), StaticSegment("bar"));
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo/bar");
        assert_eq!(matched.remaining(), "");
        let params = matched.params();
        assert!(params.is_empty());
    }

    #[test]
    fn tuple_of_static_matches_on_enum() {
        let path = "/foo/bar";
        let def = (StaticSegment(Foo), StaticSegment(Bar));
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo/bar");
        assert_eq!(matched.remaining(), "");
        let params = matched.params();
        assert!(params.is_empty());
    }

    #[test]
    fn allow_empty_match() {
        let path = "";
        let def = StaticSegment("");
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "");
        assert_eq!(matched.remaining(), "");
        let params = matched.params();
        assert!(params.is_empty());
    }

    #[test]
    fn tuple_static_mismatch() {
        let path = "/foo/baz";
        let def = (StaticSegment("foo"), StaticSegment("bar"));
        assert!(def.test(path).is_none());
    }

    #[test]
    fn tuple_static_mismatch_on_enum() {
        let path = "/foo/baz";
        let def = (StaticSegment(Foo), StaticSegment(Bar));
        assert!(def.test(path).is_none());
    }

    #[test]
    fn dont_match_smooshed_segments() {
        let path = "/foobar";
        let def = (StaticSegment(Foo), StaticSegment(Bar));
        assert!(def.test(path).is_none());
    }

    #[test]
    fn arbitrary_nesting_of_tuples_has_no_effect_on_matching() {
        let path = "/foo/bar";
        let def = (
            (),
            (StaticSegment("foo")),
            (),
            ((), ()),
            StaticSegment("bar"),
            (),
        );
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo/bar");
        assert_eq!(matched.remaining(), "");
        let params = matched.params();
        assert!(params.is_empty());
    }

    #[test]
    fn arbitrary_nesting_of_tuples_has_no_effect_on_matching_on_enum() {
        let path = "/foo/bar";
        let def = (
            (),
            (StaticSegment(Foo)),
            (),
            ((), ()),
            StaticSegment(Bar),
            (),
        );
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo/bar");
        assert_eq!(matched.remaining(), "");
        let params = matched.params();
        assert!(params.is_empty());
    }

    #[test]
    fn multi_byte_static_match() {
        let path = "/héllo/x";
        let def = StaticSegment("héllo");
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/héllo");
        assert_eq!(matched.remaining(), "/x");
        let params = matched.params();
        assert!(params.is_empty());
    }

    #[test]
    fn multi_byte_static_mismatch() {
        let def = StaticSegment("héllo");
        assert!(def.test("/hélla").is_none());
        assert!(def.test("/héll").is_none());
        assert!(def.test("/h").is_none());
    }

    #[test]
    fn mismatch_on_last_char() {
        let def = StaticSegment("pricing");
        assert!(def.test("/pricinX").is_none());
    }

    #[test]
    fn only_match_full_static_paths() {
        let def = (StaticSegment("tests"), StaticSegment("abc"));
        assert!(def.test("/tes/abc").is_none());
        assert!(def.test("/test/abc").is_none());
        assert!(def.test("/tes/abc/").is_none());
        assert!(def.test("/test/abc/").is_none());
        assert!(def.test("/tests/ab").is_none());
        assert!(def.test("/tests/ab/").is_none());
    }
}
