use std::borrow::Cow;

/// Resolves `path` relative to optional `from` and prefixes `base`.
/// `from` is the path to navigate from, `path` is the path to navigate to. `base` is the prefix for all paths.
pub fn resolve_path<'a>(
    base: &'a str,
    path: &'a str,
    from: Option<&'a str>,
) -> Cow<'a, str> {
    if has_scheme(path) {
        // don't change absolute urls
        path.into()
    } else {
        let base_path = normalize(base, false);
        let from_path = from.map(|from| normalize(from, false));

        // calculate the prefix, except if result is empty, then / is the prefix
        let result = if let Some(from_path) = from_path {
            if path.starts_with('/') {
                // if path is absolute, ignore from, use base
                base_path
            } else if from_path.find(base_path.as_ref()) != Some(0) {
                // path is not absolute and from does not start with base, prefix base and from
                base_path + from_path
            } else {
                // path is not absolute and from starts with base, prefix from (and therefore also base)
                from_path
            }
        } else {
            base_path
        };

        let result_empty = result.is_empty();
        let prefix = if result_empty { "/".into() } else { result };

        prefix + normalize(path, result_empty)
    }
}

fn has_scheme(path: &str) -> bool {
    path.starts_with("//")
        || path.starts_with("tel:")
        || path.starts_with("mailto:")
        || path
            .split_once("://")
            .map(|(prefix, _)| {
                prefix.chars().all(
                    |c: char| matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9'),
                )
            })
            .unwrap_or(false)
}

#[doc(hidden)]
/// Removes duplicate starting and ending slashes and if omit_slash is true removes the starting slash.
fn normalize(path: &str, omit_slash: bool) -> Cow<'_, str> {
    let s = path.trim_start_matches('/');
    let trim_end = s
        .chars()
        .rev()
        .take_while(|c| *c == '/')
        .count()
        .saturating_sub(1);
    let s = &s[0..s.len() - trim_end];
    if s.is_empty() || omit_slash || begins_with_query_or_hash(s) {
        s.into()
    } else {
        format!("/{s}").into()
    }
}

/// Returns whether the string starts with a `#` or `?`.
fn begins_with_query_or_hash(text: &str) -> bool {
    matches!(text.chars().next(), Some('#') | Some('?'))
}

/* TODO can remove?
#[doc(hidden)]
pub fn join_paths<'a>(from: &'a str, to: &'a str) -> String {
    let from = remove_wildcard(&normalize(from, false));
    from + normalize(to, false).as_ref()
}

fn remove_wildcard(text: &str) -> String {
    text.rsplit_once('*')
        .map(|(prefix, _)| prefix)
        .unwrap_or(text)
        .trim_end_matches('/')
        .to_string()
}
*/

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalize_query_string_with_opening_slash() {
        assert_eq!(normalize("/?foo=bar", false), "?foo=bar");
    }

    #[test]
    fn normalize_retain_trailing_slash() {
        assert_eq!(normalize("foo/bar/", false), "/foo/bar/");
    }

    #[test]
    fn normalize_dedup_trailing_slashes() {
        assert_eq!(normalize("foo/bar/////", false), "/foo/bar/");
    }
}
