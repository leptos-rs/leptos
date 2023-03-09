// Implementation based on Solid Router
// see https://github.com/solidjs/solid-router/blob/main/src/utils.ts

use std::borrow::Cow;

#[doc(hidden)]
pub fn resolve_path<'a>(
    base: &'a str,
    path: &'a str,
    from: Option<&'a str>,
) -> Option<Cow<'a, str>> {
    if has_scheme(path) {
        None
    } else {
        let base_path = normalize(base, false);
        let from_path = from.map(|from| normalize(from, false));
        let result = if let Some(from_path) = from_path {
            if path.starts_with('/') {
                base_path
            } else if from_path.to_lowercase().find(&base_path.to_lowercase())
                != Some(0)
            {
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
    let s = path.trim_start_matches('/').trim_end_matches('/');
    if s.is_empty() || omit_slash || begins_with_query_or_hash(s) {
        s.into()
    } else {
        format!("/{s}").into()
    }
}

#[doc(hidden)]
pub fn join_paths<'a>(from: &'a str, to: &'a str) -> String {
    let from = remove_wildcard(&normalize(from, false));
    from + &normalize(to, false)
}

fn begins_with_query_or_hash(text: &str) -> bool {
    matches!(text.chars().next(), Some('#') | Some('?'))
}

fn remove_wildcard(text: &str) -> String {
    text.split_once('*')
        .map(|(prefix, _)| prefix.trim_end_matches('/'))
        .unwrap_or(text)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalize_query_string_with_opening_slash() {
        assert_eq!(normalize("/?foo=bar", false), "?foo=bar");
    }
}
