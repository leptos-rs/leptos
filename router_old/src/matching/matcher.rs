// Implementation based on Solid Router
// see <https://github.com/solidjs/solid-router/blob/main/src/utils.ts>

use crate::{unescape, ParamsMap};

#[derive(Debug, Clone, PartialEq, Eq)]
#[doc(hidden)]
pub struct PathMatch {
    pub path: String,
    pub params: ParamsMap,
}

#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Matcher {
    splat: Option<String>,
    segments: Vec<String>,
    len: usize,
    partial: bool,
}

impl Matcher {
    #[doc(hidden)]
    pub fn new(path: &str) -> Self {
        Self::new_with_partial(path, false)
    }

    #[doc(hidden)]
    pub fn new_with_partial(path: &str, partial: bool) -> Self {
        let (pattern, splat) = match path.split_once("/*") {
            Some((p, s)) => (p, Some(s.to_string())),
            None => (path, None),
        };
        let segments: Vec<String> = get_segments(pattern);
        let len = segments.len();
        Self {
            splat,
            segments,
            len,
            partial,
        }
    }

    #[doc(hidden)]
    pub fn test(&self, location: &str) -> Option<PathMatch> {
        let loc_segments: Vec<&str> = get_segments(location);

        let loc_len = loc_segments.len();
        let len_diff: i32 = loc_len as i32 - self.len as i32;

        let trailing_iter = location.chars().rev().take_while(|n| *n == '/');

        // quick path: not a match if
        // 1) matcher has add'l segments not found in location
        // 2) location has add'l segments, there's no splat, and partial matches not allowed
        if loc_len < self.len
            || (len_diff > 0 && self.splat.is_none() && !self.partial)
            || (self.splat.is_none() && trailing_iter.clone().count() > 1)
        {
            None
        }
        // otherwise, start building a match
        else {
            let mut path = String::new();
            let mut params = ParamsMap::new();

            for (segment, loc_segment) in
                self.segments.iter().zip(loc_segments.iter())
            {
                if let Some(param_name) = segment.strip_prefix(':') {
                    params.insert(param_name.into(), unescape(loc_segment));
                } else if segment != loc_segment {
                    // if any segment doesn't match and isn't a param, there's no path match
                    return None;
                }

                path.push('/');
                path.push_str(loc_segment);
            }

            if let Some(splat) = &self.splat {
                if !splat.is_empty() {
                    let mut value = if len_diff > 0 {
                        loc_segments[self.len..].join("/")
                    } else {
                        "".into()
                    };

                    // add trailing slashes to splat
                    let trailing_slashes =
                        trailing_iter.skip(1).collect::<String>();
                    value.push_str(&trailing_slashes);

                    params.insert(splat.into(), value);
                }
            }

            Some(PathMatch { path, params })
        }
    }

    #[doc(hidden)]
    pub(crate) fn is_wildcard(&self) -> bool {
        self.splat.is_some()
    }
}

fn get_segments<'a, S: From<&'a str>>(pattern: &'a str) -> Vec<S> {
    // URL root paths ("/" and "") are equivalent and treated as 0-segment paths.
    // non-root paths with trailing slashes get extra empty segment at the end.
    // This makes sure that segment matching is trailing-slash sensitive.
    let mut segments: Vec<S> = pattern
        .split('/')
        .filter(|p| !p.is_empty())
        .map(Into::into)
        .collect();
    if !segments.is_empty() && pattern.ends_with('/') {
        segments.push("".into());
    }
    segments
}
