use super::{Url, BASE};
use crate::params::ParamsMap;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestUrl(Arc<str>);

impl RequestUrl {
    /// Creates a server-side request URL from a path.
    pub fn new(path: &str) -> Self {
        Self(path.into())
    }
}

impl AsRef<str> for RequestUrl {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Default for RequestUrl {
    fn default() -> Self {
        Self::new("/")
    }
}

impl RequestUrl {
    pub fn parse(&self) -> Result<Url, url::ParseError> {
        self.parse_with_base(BASE)
    }

    pub fn parse_with_base(&self, base: &str) -> Result<Url, url::ParseError> {
        let base = url::Url::parse(base)?;
        let url = url::Url::options().base_url(Some(&base)).parse(&self.0)?;

        // `url::Url::query_pairs()` already percent-decodes keys and values, so
        // we must not decode them a second time via `ParamsMap::insert`.
        let mut search_params = ParamsMap::new();
        for (k, v) in url.query_pairs() {
            search_params.insert_raw(k.into_owned(), v.into_owned());
        }

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

    #[test]
    pub fn should_parse_url_without_origin() {
        let url = RequestUrl::new("/foo/bar").parse().unwrap();
        assert_eq!(url.path(), "/foo/bar");
    }

    #[test]
    pub fn should_not_parse_url_without_slash() {
        let url = RequestUrl::new("foo/bar").parse().unwrap();
        assert_eq!(url.path(), "/foo/bar");
    }

    #[test]
    pub fn should_parse_with_base() {
        let url = RequestUrl::new("https://www.example.com/foo/bar")
            .parse()
            .unwrap();
        assert_eq!(url.origin(), "https://www.example.com");
        assert_eq!(url.path(), "/foo/bar");
    }

    #[test]
    pub fn query_param_with_literal_percent_is_decoded_exactly_once() {
        // `%2525` in the raw query string is `%25` percent-encoded, so after one
        // decode it should be `%25`, not `%` (which would be a second decode).
        let url = RequestUrl::new("/page?x=%2525").parse().unwrap();
        assert_eq!(
            url.search_params().get("x").as_deref(),
            Some("%25"),
            "expected a single decode: %2525 -> %25"
        );
    }

    #[test]
    pub fn query_param_with_regular_encoding_is_decoded() {
        // A plain percent-encoded value like `hello%20world` should still decode
        // to `hello world`.
        let url = RequestUrl::new("/page?msg=hello%20world").parse().unwrap();
        assert_eq!(
            url.search_params().get("msg").as_deref(),
            Some("hello world")
        );
    }
}
