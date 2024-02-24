use super::{PartialPathMatch, PossibleRouteMatch};
use crate::PathSegment;
use alloc::{string::String, vec::Vec};
use core::str::Chars;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ParamSegment(pub &'static str);

impl PossibleRouteMatch for ParamSegment {
    fn matches_iter(&self, test: &mut Chars) -> bool {
        let mut test = test.peekable();
        // match an initial /
        if test.peek() == Some(&'/') {
            test.next();
        }
        for char in test {
            // when we get a closing /, stop matching
            if char == '/' {
                break;
            }
        }
        true
    }

    fn test<'a>(&self, path: &'a str) -> Option<PartialPathMatch<'a>> {
        let mut matched = String::new();
        let mut param_value = String::new();
        let mut test = path.chars();

        // match an initial /
        if let Some('/') = test.next() {
            matched.push('/');
        }
        for char in test {
            // when we get a closing /, stop matching
            if char == '/' {
                break;
            }
            // otherwise, push into the matched param
            else {
                matched.push(char);
                param_value.push(char);
            }
        }

        let next_index = matched.len();
        Some(PartialPathMatch::new(
            &path[next_index..],
            vec![(self.0, param_value)],
            matched,
        ))
    }

    fn generate_path(&self, path: &mut Vec<PathSegment>) {
        path.push(PathSegment::Param(self.0.into()));
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct WildcardSegment(pub &'static str);

impl PossibleRouteMatch for WildcardSegment {
    fn matches_iter(&self, _path: &mut Chars) -> bool {
        true
    }

    fn test<'a>(&self, path: &'a str) -> Option<PartialPathMatch<'a>> {
        let mut matched = String::new();
        let mut param_value = String::new();
        let mut test = path.chars();

        // match an initial /
        if let Some('/') = test.next() {
            matched.push('/');
        }
        for char in test {
            matched.push(char);
            param_value.push(char);
        }

        let next_index = matched.len();
        Some(PartialPathMatch::new(
            &path[next_index..],
            vec![(self.0, param_value)],
            matched,
        ))
    }

    fn generate_path(&self, path: &mut Vec<PathSegment>) {
        path.push(PathSegment::Splat(self.0.into()));
    }
}

#[cfg(test)]
mod tests {
    use super::PossibleRouteMatch;
    use crate::matching::{ParamSegment, StaticSegment, WildcardSegment};
    use alloc::string::ToString;

    #[test]
    fn single_param_match() {
        let path = "/foo";
        let def = ParamSegment("a");
        assert!(def.matches(path));
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo");
        assert_eq!(matched.remaining(), "");
        assert_eq!(matched.params()[0], ("a", "foo".to_string()));
    }

    #[test]
    fn single_param_match_with_trailing_slash() {
        let path = "/foo/";
        let def = ParamSegment("a");
        assert!(def.matches(path));
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo");
        assert_eq!(matched.remaining(), "/");
        assert_eq!(matched.params()[0], ("a", "foo".to_string()));
    }

    #[test]
    fn tuple_of_param_matches() {
        let path = "/foo/bar";
        let def = (ParamSegment("a"), ParamSegment("b"));
        assert!(def.matches(path));
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo/bar");
        assert_eq!(matched.remaining(), "");
        assert_eq!(matched.params()[0], ("a", "foo".to_string()));
        assert_eq!(matched.params()[1], ("b", "bar".to_string()));
    }

    #[test]
    fn splat_should_match_all() {
        let path = "/foo/bar/////";
        let def = (
            StaticSegment("foo"),
            StaticSegment("bar"),
            WildcardSegment("rest"),
        );
        assert!(def.matches(path));
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo/bar/////");
        assert_eq!(matched.remaining(), "");
        assert_eq!(matched.params()[0], ("rest", "////".to_string()));
    }
}
