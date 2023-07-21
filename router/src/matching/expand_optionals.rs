use std::borrow::Cow;

#[doc(hidden)]
#[cfg(not(feature = "ssr"))]
pub fn expand_optionals(pattern: &str) -> Vec<Cow<str>> {
    use js_sys::RegExp;
    use once_cell::unsync::Lazy;
    use wasm_bindgen::JsValue;

    thread_local! {
        static OPTIONAL_RE: Lazy<RegExp> = Lazy::new(|| {
            RegExp::new(OPTIONAL, "")
        });
        static OPTIONAL_RE_2: Lazy<RegExp> = Lazy::new(|| {
            RegExp::new(OPTIONAL_2, "")
        });
    }

    let captures = OPTIONAL_RE.with(|re| re.exec(pattern));
    match captures {
        None => vec![pattern.into()],
        Some(matched) => {
            let start: usize =
                js_sys::Reflect::get(&matched, &JsValue::from_str("index"))
                    .unwrap()
                    .as_f64()
                    .unwrap() as usize;
            let mut prefix = pattern[0..start].to_string();
            let mut suffix =
                &pattern[start + matched.get(1).as_string().unwrap().len()..];
            let mut prefixes = vec![prefix.clone()];

            prefix += &matched.get(1).as_string().unwrap();
            prefixes.push(prefix.clone());

            while let Some(matched) =
                OPTIONAL_RE_2.with(|re| re.exec(suffix.trim_start_matches('?')))
            {
                prefix += &matched.get(1).as_string().unwrap();
                prefixes.push(prefix.clone());
                suffix = &suffix[matched.get(0).as_string().unwrap().len()..];
            }

            expand_optionals(suffix).iter().fold(
                Vec::new(),
                |mut results, expansion| {
                    results.extend(prefixes.iter().map(|prefix| {
                        Cow::Owned(
                            prefix.clone() + expansion.trim_start_matches('?'),
                        )
                    }));
                    results
                },
            )
        }
    }
}

#[doc(hidden)]
#[cfg(feature = "ssr")]
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

            while let Some(captures) =
                OPTIONAL_RE_2.captures(suffix.trim_start_matches('?'))
            {
                prefix += &captures[1];
                prefixes.push(prefix.clone());
                suffix = &suffix[captures[0].len()..];
            }

            expand_optionals(suffix).iter().fold(
                Vec::new(),
                |mut results, expansion| {
                    results.extend(prefixes.iter().map(|prefix| {
                        Cow::Owned(
                            prefix.clone() + expansion.trim_start_matches('?'),
                        )
                    }));
                    results
                },
            )
        }
    }
}

const OPTIONAL: &str = r"(/?:[^/]+)\?";
const OPTIONAL_2: &str = r"^(/:[^/]+)\?";
