use std::borrow::Cow;

pub fn resolve_path<'a>(
    base: &'a str,
    path: &'a str,
    from: Option<&'a str>,
) -> Cow<'a, str> {
    if has_scheme(path) {
        path.into()
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

        prefix + normalize(path, result_empty)
    }
}

fn has_scheme(path: &str) -> bool {
    // Protocol-relative URLs.
    if path.starts_with("//") {
        return true;
    }
    // RFC 3986 §3.1: scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )
    // This covers both authority-based schemes (`https://`) and opaque ones
    // (`mailto:`, `tel:`, `javascript:`, `data:`, `view-source:`), as well as
    // compound schemes such as `git+ssh` that the previous check rejected.
    let Some((scheme, _)) = path.split_once(':') else {
        return false;
    };
    let mut chars = scheme.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_alphabetic()
        && chars
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
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

    #[test]
    fn has_scheme_detects_authority_and_opaque_schemes() {
        // Authority-based schemes.
        assert!(has_scheme("https://example.com"));
        assert!(has_scheme("ws://x/sock"));
        assert!(has_scheme("//example.com"));
        // Opaque schemes (no `://`) that the previous check missed.
        assert!(has_scheme("mailto:foo@bar.com"));
        assert!(has_scheme("tel:+15551234"));
        assert!(has_scheme("javascript:alert(1)"));
        assert!(has_scheme("data:text/html,<h1>x</h1>"));
        assert!(has_scheme("view-source:https://example.com"));
        assert!(has_scheme("blob:https://example.com/abc"));
        // Compound schemes with `+`/`-`/`.` that the previous filter rejected.
        assert!(has_scheme("git+ssh://host/x.git"));
        assert!(has_scheme("coap+tcp://host"));
    }

    #[test]
    fn has_scheme_rejects_relative_paths() {
        assert!(!has_scheme("/foo/bar"));
        assert!(!has_scheme("foo/bar"));
        assert!(!has_scheme("?query=1"));
        assert!(!has_scheme("#fragment"));
        // A scheme must start with a letter, not a digit.
        assert!(!has_scheme("1foo:bar"));
        // Empty scheme.
        assert!(!has_scheme(":nothing"));
    }

    #[test]
    fn resolve_path_leaves_opaque_schemes_untouched() {
        assert_eq!(
            resolve_path("/", "javascript:alert(1)", None),
            "javascript:alert(1)"
        );
        assert_eq!(
            resolve_path("/", "data:text/html,<h1>x</h1>", None),
            "data:text/html,<h1>x</h1>"
        );
    }
}
