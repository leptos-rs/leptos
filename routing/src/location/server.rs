use super::{Location, LocationChange, Url};
use alloc::string::{String, ToString};
use core::fmt::Display;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestUrl(String);

impl RequestUrl {
    /// Creates a server-side request URL from a path.
    pub fn new(path: impl Display) -> Self {
        Self(path.to_string())
    }
}

impl Default for RequestUrl {
    fn default() -> Self {
        Self(String::from("/"))
    }
}

impl Location for RequestUrl {
    type Error = url::ParseError;

    fn init(&self) {}

    fn set_navigation_hook(&mut self, _cb: impl FnMut(Url) + 'static) {}

    fn navigate(&self, _loc: &LocationChange) {}

    fn parse_with_base(url: &str, base: &str) -> Result<Url, Self::Error> {
        let base = url::Url::parse(base)?;
        let url = url::Url::options().base_url(Some(&base)).parse(&url)?;

        let search_params = url
            .query_pairs()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        Ok(Url {
            origin: url.origin().unicode_serialization(),
            path: url.path().to_string(),
            search: url.query().unwrap_or_default().to_string(),
            search_params,
            hash: Default::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::RequestUrl;
    use crate::location::Location;

    #[test]
    pub fn should_parse_url_without_origin() {
        let url = RequestUrl::parse("/foo/bar").unwrap();
        assert_eq!(url.path(), "/foo/bar");
    }

    #[test]
    pub fn should_not_parse_url_without_slash() {
        let url = RequestUrl::parse("foo/bar").unwrap();
        assert_eq!(url.path(), "/foo/bar");
    }

    #[test]
    pub fn should_parse_with_base() {
        let url = RequestUrl::parse("https://www.example.com/foo/bar").unwrap();
        assert_eq!(url.origin(), "https://www.example.com");
        assert_eq!(url.path(), "/foo/bar");
    }
}
