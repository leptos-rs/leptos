use std::borrow::Cow;

#[doc(hidden)]
/* pub fn expand_optionals(pattern: &str) -> impl Iterator<Item = Cow<str>> {
    todo!()
} */

const CONTAINS_OPTIONAL: &str = r#"(/?\:[^\/]+)\?"#;
