use super::{PartialPathMatch, PossibleRouteMatch};
use crate::PathSegment;
use alloc::vec::Vec;

impl PossibleRouteMatch for () {
    fn matches<'a>(&self, path: &'a str) -> Option<&'a str> {
        Some(path)
    }

    fn test<'a>(&self, path: &'a str) -> Option<PartialPathMatch<'a>> {
        Some(PartialPathMatch::new(path, [], ""))
    }

    fn generate_path(&self, _path: &mut Vec<PathSegment>) {}
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct StaticSegment(pub &'static str);

impl PossibleRouteMatch for StaticSegment {
    fn matches<'a>(&self, path: &'a str) -> Option<&'a str> {
        let mut matched_len = 0;
        let mut this = self.0.chars();
        let mut test = path.chars().peekable();
        if test.peek() == Some(&'/') {
            matched_len += '/'.len_utf8();
            test.next();
        }

        // unless this segment is empty, we start by
        // assuming that it has not actually matched
        let mut has_matched = self.0.is_empty() || self.0 == "/";
        if !self.0.is_empty() {
            for char in test {
                // when we get a closing /, stop matching
                if char == '/' {
                    break;
                }
                // if the next character in the path doesn't match the
                // next character in the segment, we don't match
                else if this.next() != Some(char) {
                    return None;
                } else {
                    matched_len += char.len_utf8();
                    has_matched = true;
                }
            }
        }
        println!("matching on {self:?}, has_matched = {has_matched}");

        has_matched.then(|| &path[matched_len..])
    }

    fn test<'a>(&self, path: &'a str) -> Option<PartialPathMatch<'a>> {
        let mut matched_len = 0;
        let mut test = path.chars();
        let mut this = self.0.chars();
        let mut has_matched = self.0.is_empty() || self.0 == "/";

        // match an initial /
        if let Some('/') = test.next() {
            if !self.0.is_empty() {
                matched_len += 1;
            }
        }
        for char in test {
            let n = this.next();
            // when we get a closing /, stop matching
            if char == '/' || n.is_none() {
                break;
            }
            // if the next character in the path matches the
            // next character in the segment, add it to the match
            else if Some(char) == n {
                has_matched = true;
                matched_len += char.len_utf8();
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
        let (matched, remaining) = path.split_at(matched_len);
        has_matched.then(|| PartialPathMatch::new(remaining, [], matched))
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
        assert!(def.matches(path).is_some());
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo");
        assert_eq!(matched.remaining(), "/");
        assert!(matched.params().is_empty());
    }

    #[test]
    fn tuple_of_static_matches() {
        let path = "/foo/bar";
        let def = (StaticSegment("foo"), StaticSegment("bar"));
        assert!(def.matches(path).is_some());
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo/bar");
        assert_eq!(matched.remaining(), "");
        assert!(matched.params().is_empty());
    }

    #[test]
    fn tuple_static_mismatch() {
        let path = "/foo/baz";
        let def = (StaticSegment("foo"), StaticSegment("bar"));
        assert!(def.matches(path).is_none());
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
        assert!(def.matches(path).is_some());
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo/bar");
        assert_eq!(matched.remaining(), "");
        assert!(matched.params().is_empty());
    }
}
