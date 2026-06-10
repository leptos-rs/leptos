use super::{BASE, Url};
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

        // `query_pairs()` already percent-decodes the values, so insert them
        // as-is. Going through `ParamsMap::insert`/`FromIterator` would decode
        // a second time and corrupt any literal `%xx` sequence.
        let mut search_params = ParamsMap::new();
        for (k, v) in url.query_pairs() {
            search_params.insert_decoded(k.into_owned(), v.into_owned());
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
    pub fn should_decode_query_params_exactly_once() {
        // `%2520` decodes once to `%20`; a second (buggy) decode would turn it
        // into a space.
        let url = RequestUrl::new("/?q=100%2520percent").parse().unwrap();
        assert_eq!(url.search_params().get("q").unwrap(), "100%20percent");

        // A literal percent-encoded slash inside an opaque id must survive.
        let url = RequestUrl::new("/api?id=foo%252Fbar").parse().unwrap();
        assert_eq!(url.search_params().get("id").unwrap(), "foo%2Fbar");

        // Base64 padding carried in a signature must not be truncated.
        let url = RequestUrl::new("/?sig=YWJj%253D%253D").parse().unwrap();
        assert_eq!(url.search_params().get("sig").unwrap(), "YWJj%3D%3D");
    }
}
