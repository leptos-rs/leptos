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

#[cfg(feature = "ssr")]
fn has_scheme(path: &str) -> bool {
    use regex::Regex;
    lazy_static::lazy_static! {
        pub static ref HAS_SCHEME_RE: Regex =
            Regex::new(HAS_SCHEME).expect("couldn't compile HAS_SCHEME_RE");
    }

    HAS_SCHEME_RE.is_match(path)
}

#[cfg(not(feature = "ssr"))]
fn has_scheme(path: &str) -> bool {
    let re = js_sys::RegExp::new(HAS_SCHEME, "");
    re.test(path)
}

#[doc(hidden)]
pub fn normalize(path: &str, omit_slash: bool) -> Cow<'_, str> {
    let s = replace_trim_path(path, "");
    if !s.is_empty() {
        if omit_slash || begins_with_query_or_hash(&s) {
            s
        } else {
            format!("/{s}").into()
        }
    } else {
        "".into()
    }
}

#[doc(hidden)]
pub fn join_paths<'a>(from: &'a str, to: &'a str) -> String {
    let from = replace_query(&normalize(from, false));
    from + &normalize(to, false)
}

const TRIM_PATH: &str = r#"^/+|/+$"#;
const BEGINS_WITH_QUERY_OR_HASH: &str = r#"^[?#]"#;
const HAS_SCHEME: &str = r#"^(?:[a-z0-9]+:)?//"#;
const QUERY: &str = r#"/*(\*.*)?$"#;

#[cfg(not(feature = "ssr"))]
fn replace_trim_path<'a>(text: &'a str, replace: &str) -> Cow<'a, str> {
    let re = js_sys::RegExp::new(TRIM_PATH, "g");
    js_sys::JsString::from(text)
        .replace_by_pattern(&re, replace)
        .as_string()
        .unwrap()
        .into()
}

#[cfg(not(feature = "ssr"))]
fn begins_with_query_or_hash(text: &str) -> bool {
    let re = js_sys::RegExp::new(BEGINS_WITH_QUERY_OR_HASH, "");
    re.test(text)
}

#[cfg(not(feature = "ssr"))]
fn replace_query(text: &str) -> String {
    let re = js_sys::RegExp::new(QUERY, "g");
    js_sys::JsString::from(text)
        .replace_by_pattern(&re, "")
        .as_string()
        .unwrap()
}

#[cfg(feature = "ssr")]
fn replace_trim_path<'a>(text: &'a str, replace: &str) -> Cow<'a, str> {
    use regex::Regex;
    lazy_static::lazy_static! {
        pub static ref TRIM_PATH_RE: Regex =
            Regex::new(TRIM_PATH).expect("couldn't compile TRIM_PATH_RE");
    }

    TRIM_PATH_RE.replace(text, replace)
}

#[cfg(feature = "ssr")]
fn begins_with_query_or_hash(text: &str) -> bool {
    use regex::Regex;
    lazy_static::lazy_static! {
        pub static ref BEGINS_WITH_QUERY_OR_HASH_RE: Regex =
            Regex::new(BEGINS_WITH_QUERY_OR_HASH).expect("couldn't compile BEGINS_WITH_HASH_RE");
    }
    BEGINS_WITH_QUERY_OR_HASH_RE.is_match(text)
}

#[cfg(feature = "ssr")]
fn replace_query(text: &str) -> String {
    use regex::Regex;
    lazy_static::lazy_static! {
        pub static ref QUERY_RE: Regex =
            Regex::new(QUERY).expect("couldn't compile QUERY_RE");
    }
    QUERY_RE.replace(text, "").into_owned()
}
