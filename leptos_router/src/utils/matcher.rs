// Implementation based on Solid Router
// see https://github.com/solidjs/solid-router/blob/main/src/utils.ts

use std::borrow::Cow;

use crate::Params;

#[derive(Debug, PartialEq, Eq)]
#[doc(hidden)]
pub struct PathMatch<'a> {
    pub path: Cow<'a, str>,
    pub params: Params,
}

#[doc(hidden)]
pub struct Matcher<'a> {
    splat: Option<&'a str>,
    segments: Vec<&'a str>,
    len: usize,
    partial: bool,
}

impl<'a> Matcher<'a> {
    #[doc(hidden)]
    pub fn new(path: &'a str) -> Self {
        Self::new_with_partial(path, false)
    }

    #[doc(hidden)]
    pub fn new_with_partial(path: &'a str, partial: bool) -> Self {
        let (pattern, splat) = match path.split_once("/*") {
            Some((p, s)) => (p, Some(s)),
            None => (path, None),
        };
        let segments = pattern
            .split('/')
            .filter(|n| !n.is_empty())
            .collect::<Vec<_>>();

        let len = segments.len();

        Self {
            splat,
            segments,
            len,
            partial,
        }
    }

    #[doc(hidden)]
    pub fn test<'b>(&self, location: &'b str) -> Option<PathMatch<'b>>
    where
        'a: 'b,
    {
        let loc_segments = location
            .split('/')
            .filter(|n| !n.is_empty())
            .collect::<Vec<_>>();
        let loc_len = loc_segments.len();
        let len_diff = loc_len - self.len;

        // quick path: not a match if
        // 1) matcher has add'l segments not found in location
        // 2) location has add'l segments, there's no splat, and partial matches not allowed
        if loc_len < self.len || (len_diff > 0 && self.splat.is_none() && !self.partial) {
            None
        }
        // otherwise, start building a match
        else {
            /* let matched = PathMatch {
                path: if self.len > 0 {
                    "".into()
                } else {
                    "/".into()
                },
                params: Params::new()
            }; */

            let mut path = String::new();
            let mut params = Params::new();
            for (segment, loc_segment) in self.segments.iter().zip(loc_segments.iter()) {
                if let Some(param_name) = segment.strip_prefix(':') {
                    params.insert(param_name.into(), (*loc_segment).into());
                } else if segment != loc_segment {
                    // if any segment doesn't match and isn't a param, there's no path match
                    return None;
                }

                path.push('/');
                path.push_str(loc_segment);
            }

            if let Some(splat) = self.splat && !splat.is_empty() {
                let value = if len_diff > 0 {
                    loc_segments[self.len..].join("/").into()
                } else {
                    "".into()
                };
                params.insert(splat.into(), value);
            }

            Some(PathMatch {
                path: path.into(),
                params,
            })
        }
    }
}
