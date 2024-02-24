use super::{PartialPathMatch, PossibleRouteMatch};
use crate::PathSegment;
use alloc::{string::String, vec::Vec};
use core::str::Chars;

impl PossibleRouteMatch for () {
    fn test<'a>(&self, path: &'a str) -> Option<PartialPathMatch<'a>> {
        Some(PartialPathMatch::new(path, [], ""))
    }

    fn matches_iter(&self, _path: &mut Chars) -> bool {
        true
    }

    fn generate_path(&self, _path: &mut Vec<PathSegment>) {}
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct StaticSegment(pub &'static str);

impl PossibleRouteMatch for StaticSegment {
    fn matches_iter(&self, test: &mut Chars) -> bool {
        let mut this = self.0.chars();
        let mut test = test.peekable();
        if test.peek() == Some(&'/') {
            test.next();
        }

        // unless this segment is empty, we start by
        // assuming that it has not actually matched
        let mut has_matched = self.0.is_empty();
        for char in test {
            // when we get a closing /, stop matching
            if char == '/' {
                break;
            }
            // if the next character in the path doesn't match the
            // next character in the segment, we don't match
            else if this.next() != Some(char) {
                return false;
            } else {
                has_matched = true;
            }
        }

        has_matched
    }

    fn test<'a>(&self, path: &'a str) -> Option<PartialPathMatch<'a>> {
        let mut matched = String::new();
        let mut test = path.chars();
        let mut this = self.0.chars();

        // match an initial /
        if let Some('/') = test.next() {
            matched.push('/');
        }
        for char in test {
            // when we get a closing /, stop matching
            if char == '/' {
                break;
            }
            // if the next character in the path matches the
            // next character in the segment, add it to the match
            else if Some(char) == this.next() {
                matched.push(char);
            }
            // otherwise, this route doesn't match and we should
            // return None
            else {
                return None;
            }
        }

        // build the match object
        // the remaining is built from the path in, with the slice moved
        // by the length of this match
        let next_index = matched.len();
        Some(PartialPathMatch::new(
            &path[next_index..],
            Vec::new(),
            matched,
        ))
    }

    fn generate_path(&self, path: &mut Vec<PathSegment>) {
        path.push(PathSegment::Static(self.0.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::{PossibleRouteMatch, StaticSegment};

    #[test]
    fn single_static_match() {
        let path = "/foo";
        let def = StaticSegment("foo");
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo");
        assert_eq!(matched.remaining(), "");
        assert!(matched.params().is_empty());
    }

    #[test]
    fn single_static_mismatch() {
        let path = "/foo";
        let def = StaticSegment("bar");
        assert!(def.test(path).is_none());
    }

    #[test]
    fn single_static_match_with_trailing_slash() {
        let path = "/foo/";
        let def = StaticSegment("foo");
        assert!(def.matches(path));
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo");
        assert_eq!(matched.remaining(), "/");
        assert!(matched.params().is_empty());
    }

    #[test]
    fn tuple_of_static_matches() {
        let path = "/foo/bar";
        let def = (StaticSegment("foo"), StaticSegment("bar"));
        assert!(def.matches(path));
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo/bar");
        assert_eq!(matched.remaining(), "");
        assert!(matched.params().is_empty());
    }

    #[test]
    fn tuple_static_mismatch() {
        let path = "/foo/baz";
        let def = (StaticSegment("foo"), StaticSegment("bar"));
        assert!(!def.matches(path));
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
        assert!(def.matches(path));
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo/bar");
        assert_eq!(matched.remaining(), "");
        assert!(matched.params().is_empty());
    }
}
