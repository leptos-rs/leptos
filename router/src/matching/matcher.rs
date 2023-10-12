// Implementation based on Solid Router
// see <https://github.com/solidjs/solid-router/blob/main/src/utils.ts>

use crate::ParamsMap;

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
        let segments = pattern
            .split('/')
            .map(|n| n.to_string())
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
    pub fn test(&self, location: &str) -> Option<PathMatch> {
        let loc_segments = location
            .split('/')
            .filter(|n| !n.is_empty())
            .collect::<Vec<_>>();

        let loc_len = loc_segments.len();
        let len_diff: i32 = loc_len as i32 - self.len as i32;

        leptos::logging::log!("{:?}\n{loc_segments:?}", self.segments);

        // quick path: not a match if
        // 1) matcher has add'l segments not found in location
        // 2) location has add'l segments, there's no splat, and partial matches not allowed
        if loc_len < self.len
            || (len_diff > 0 && self.splat.is_none() && !self.partial)
        {
            None
        }
        // otherwise, start building a match
        else {
            let mut path = String::new();
            let mut params = ParamsMap::new();

            let segments = self.segments.iter();
            let mut loc_segments_iter = loc_segments.iter();
            for segment in segments {
                // if it's a splat, move on
                if segment.starts_with('*') {
                    _ = loc_segments_iter.next();
                    break;
                }

                if let Some(loc_segment) = loc_segments_iter.next() {
                    if let Some(param_name) = segment.strip_prefix(':') {
                        params.insert(param_name.into(), (*loc_segment).into());
                    } else if segment != loc_segment {
                        // if any segment doesn't match and isn't a param, there's no path match
                        return None;
                    }

                    if !segment.is_empty() {
                        path.push('/');
                    }
                    path.push_str(loc_segment);
                }
            }

            if let Some(splat) = &self.splat {
                if !splat.is_empty() {
                    let mut value = String::new();
                    _ = loc_segments_iter.next(); // eat the next `/`
                    for loc_segment in loc_segments_iter {
                        value.push('/');
                        value.push_str(loc_segment);
                    }
                    params.insert(splat.into(), value);
                }
            }

            Some(PathMatch { path, params })
        }
    }
}
