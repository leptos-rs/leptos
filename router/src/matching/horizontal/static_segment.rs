use super::{PartialPathMatch, PathSegment, PossibleRouteMatch};
use core::iter;
use std::{borrow::Cow, fmt::Debug};

impl PossibleRouteMatch for () {
    type ParamsIter = iter::Empty<(Cow<'static, str>, String)>;

    fn test<'a>(
        &self,
        path: &'a str,
    ) -> Option<PartialPathMatch<'a, Self::ParamsIter>> {
        Some(PartialPathMatch::new(path, iter::empty(), ""))
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct StaticSegment<T: AsPath>(pub T);

impl<T: AsPath> PossibleRouteMatch for StaticSegment<T> {
    type ParamsIter = iter::Empty<(Cow<'static, str>, String)>;

    fn test<'a>(
        &self,
        path: &'a str,
    ) -> Option<PartialPathMatch<'a, Self::ParamsIter>> {
        let mut matched_len = 0;
        let mut test = path.chars().peekable();
        let mut this = self.0.as_path().chars();
        let mut has_matched =
            self.0.as_path().is_empty() || self.0.as_path() == "/";

        // match an initial /
        if let Some('/') = test.peek() {
            test.next();

            if !self.0.as_path().is_empty() {
                matched_len += 1;
            }
            if self.0.as_path().starts_with('/') || self.0.as_path().is_empty()
            {
                this.next();
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
        has_matched
            .then(|| PartialPathMatch::new(remaining, iter::empty(), matched))
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
        let params = matched.params().collect::<Vec<_>>();
        assert!(params.is_empty());
    }

    #[test]
    fn single_static_match_on_enum() {
        let path = "/foo";
        let def = StaticSegment(Foo);
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo");
        assert_eq!(matched.remaining(), "");
        let params = matched.params().collect::<Vec<_>>();
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
        let params = matched.params().collect::<Vec<_>>();
        assert!(params.is_empty());
    }

    #[test]
    fn single_static_match_with_trailing_slash_on_enum() {
        let path = "/foo/";
        let def = StaticSegment(Foo);
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo");
        assert_eq!(matched.remaining(), "/");
        let params = matched.params().collect::<Vec<_>>();
        assert!(params.is_empty());
    }

    #[test]
    fn tuple_of_static_matches() {
        let path = "/foo/bar";
        let def = (StaticSegment("foo"), StaticSegment("bar"));
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo/bar");
        assert_eq!(matched.remaining(), "");
        let params = matched.params().collect::<Vec<_>>();
        assert!(params.is_empty());
    }

    #[test]
    fn tuple_of_static_matches_on_enum() {
        let path = "/foo/bar";
        let def = (StaticSegment(Foo), StaticSegment(Bar));
        let matched = def.test(path).expect("couldn't match route");
        assert_eq!(matched.matched(), "/foo/bar");
        assert_eq!(matched.remaining(), "");
        let params = matched.params().collect::<Vec<_>>();
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
        let params = matched.params().collect::<Vec<_>>();
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
        let params = matched.params().collect::<Vec<_>>();
        assert!(params.is_empty());
    }
}
