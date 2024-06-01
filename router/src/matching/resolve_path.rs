use std::borrow::Cow;

pub fn resolve_path<'a>(
    base: &'a str,
    path: &'a str,
    from: Option<&'a str>,
) -> Option<Cow<'a, str>> {
    if has_scheme(path) {
        Some(path.into())
    } else {
        let base_path = normalize(base, false);
        let from_path = from.map(|from| normalize(from, false));
        let result = if let Some(from_path) = from_path {
            if path.starts_with('/') {
                base_path
            } else if from_path.find(base_path.as_ref()) != Some(0) {
                base_path + from_path
            } else {
                from_path
            }
        } else {
            base_path
        };

        let result_empty = result.is_empty();
        let prefix = if result_empty { "/".into() } else { result };

        Some(prefix + normalize(path, result_empty))
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
