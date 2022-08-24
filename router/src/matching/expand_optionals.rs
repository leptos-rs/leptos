use std::borrow::Cow;

#[doc(hidden)]
#[cfg(feature = "browser")]
pub fn expand_optionals(pattern: &str) -> Vec<Cow<str>> {
    // TODO real implementation for browser
    vec![pattern.into()]
}

#[doc(hidden)]
#[cfg(not(feature = "browser"))]
pub fn expand_optionals(pattern: &str) -> Vec<Cow<str>> {
    use regex::Regex;

    lazy_static::lazy_static! {
        pub static ref OPTIONAL_RE: Regex = Regex::new(OPTIONAL).expect("could not compile OPTIONAL_RE");
        pub static ref OPTIONAL_RE_2: Regex = Regex::new(OPTIONAL_2).expect("could not compile OPTIONAL_RE_2");
    }

    let captures = OPTIONAL_RE.find(pattern);
    match captures {
        None => vec![pattern.into()],
        Some(matched) => {
            let mut prefix = pattern[0..matched.start()].to_string();
            let captures = OPTIONAL_RE.captures(pattern).unwrap();
            let mut suffix = &pattern[matched.start() + captures[1].len()..];
            let mut prefixes = vec![prefix.clone()];

            prefix += &captures[1];
            prefixes.push(prefix.clone());

            while let Some(captures) = OPTIONAL_RE_2.captures(suffix.trim_start_matches('?')) {
                prefix += &captures[1];
                prefixes.push(prefix.clone());
                suffix = &suffix[captures[0].len()..];
            }

            expand_optionals(suffix)
                .iter()
                .fold(Vec::new(), |mut results, expansion| {
                    results.extend(prefixes.iter().map(|prefix| {
                        Cow::Owned(prefix.clone() + expansion.trim_start_matches('?'))
                    }));
                    results
                })
        }
    }
}

const OPTIONAL: &str = r#"(/?:[^/]+)\?"#;
const OPTIONAL_2: &str = r#"^(/:[^/]+)\?"#;
