use super::{PartialPathMatch, PossibleRouteMatch};
use crate::PathSegment;
use alloc::vec::Vec;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ParamSegment(pub &'static str);

impl PossibleRouteMatch for ParamSegment {
    fn matches<'a>(&self, path: &'a str) -> Option<&'a str> {
        let mut matched_len = 0;
        let mut test = path.chars().peekable();
        // match an initial /
        if test.peek() == Some(&'/') {
            matched_len += 1;
            test.next();
        }
        for char in test {
            // when we get a closing /, stop matching
            if char == '/' {
                break;
            }
            matched_len += char.len_utf8();
        }
        Some(&path[0..matched_len])
    }

    fn test<'a>(&self, path: &'a str) -> Option<PartialPathMatch<'a>> {
        let mut matched_len = 0;
        let mut param_offset = 0;
        let mut param_len = 0;
        let mut test = path.chars();

        // match an initial /
        if let Some('/') = test.next() {
            matched_len += 1;
            param_offset = 1;
        }
        for char in test {
            // when we get a closing /, stop matching
            if char == '/' {
                break;
            }
            // otherwise, push into the matched param
            else {
                matched_len += char.len_utf8();
                param_len += char.len_utf8();
            }
        }

        let (matched, remaining) = path.split_at(matched_len);
        let param_value =
            vec![(self.0, &path[param_offset..param_len + param_offset])];
        Some(PartialPathMatch::new(remaining, param_value, matched))
    }

    fn generate_path(&self, path: &mut Vec<PathSegment>) {
        path.push(PathSegment::Param(self.0.into()));
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct WildcardSegment(pub &'static str);

impl PossibleRouteMatch for WildcardSegment {
    fn matches<'a>(&self, path: &'a str) -> Option<&'a str> {
        Some(path)
    }

    fn test<'a>(&self, path: &'a str) -> Option<PartialPathMatch<'a>> {
        let mut matched_len = 0;
        let mut param_offset = 0;
        let mut param_len = 0;
        let mut test = path.chars();

        // match an initial /
        if let Some('/') = test.next() {
            matched_len += 1;
            param_offset += 1;
        }
        for char in test {
            matched_len += char.len_utf8();
            param_len += char.len_utf8();
        }

        let (matched, remaining) = path.split_at(matched_len);
        let param_value =
            vec![(self.0, &path[param_offset..param_len + param_offset])];
        Some(PartialPathMatch::new(remaining, param_value, matched))
    }

    fn generate_path(&self, path: &mut Vec<PathSegment>) {
        path.push(PathSegment::Splat(self.0.into()));
    }
}

#[cfg(test)]
mod tests {
    use super::PossibleRouteMatch;
    use crate::matching::{ParamSegment, StaticSegment, WildcardSegment};

    #[test]
    fn single_param_match() {
        let path = "/foo";
        let def = ParamSegment("a");
        assert!(def.matches(path).is_some());
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo");
        assert_eq!(matched.remaining(), "");
        assert_eq!(matched.params()[0], ("a", "foo"));
    }

    #[test]
    fn single_param_match_with_trailing_slash() {
        let path = "/foo/";
        let def = ParamSegment("a");
        assert!(def.matches(path).is_some());
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo");
        assert_eq!(matched.remaining(), "/");
        assert_eq!(matched.params()[0], ("a", "foo"));
    }

    #[test]
    fn tuple_of_param_matches() {
        let path = "/foo/bar";
        let def = (ParamSegment("a"), ParamSegment("b"));
        assert!(def.matches(path).is_some());
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo/bar");
        assert_eq!(matched.remaining(), "");
        assert_eq!(matched.params()[0], ("a", "foo"));
        assert_eq!(matched.params()[1], ("b", "bar"));
    }

    #[test]
    fn splat_should_match_all() {
        let path = "/foo/bar/////";
        let def = (
            StaticSegment("foo"),
            StaticSegment("bar"),
            WildcardSegment("rest"),
        );
        assert!(def.matches(path).is_some());
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo/bar/////");
        assert_eq!(matched.remaining(), "");
        assert_eq!(matched.params()[0], ("rest", "////"));
    }
}
