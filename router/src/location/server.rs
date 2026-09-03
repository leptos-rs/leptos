use super::Url;
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
    /// Parses the request URL into its components.
    ///
    /// A server-side request URL is normally a relative reference such as
    /// `/foo?bar`, with no scheme or authority. `url::Url` is an *absolute*-URL
    /// type and cannot decompose a relative reference without a base, so the
    /// path and query are parsed by hand here — no base and no synthetic
    /// authority are required. Inputs that already carry a scheme (a full
    /// absolute URL, which some proxies forward) are delegated to `url::Url`.
    pub fn parse(&self) -> Result<Url, url::ParseError> {
        match url::Url::parse(&self.0) {
            Ok(url) => Ok(Self::from_absolute(url)),
            Err(url::ParseError::RelativeUrlWithoutBase) => {
                Ok(self.parse_relative())
            }
            Err(e) => Err(e),
        }
    }

    fn from_absolute(url: url::Url) -> Url {
        // `query_pairs()` already percent-decodes the values, so insert them
        // as-is. Going through `ParamsMap::insert`/`FromIterator` would decode
        // a second time and corrupt any literal `%xx` sequence.
        let mut search_params = ParamsMap::new();
        for (k, v) in url.query_pairs() {
            search_params.insert_decoded(k.into_owned(), v.into_owned());
        }

        Url {
            origin: url.origin().unicode_serialization(),
            path: url.path().to_string(),
            search: url.query().unwrap_or_default().to_string(),
            search_params,
            hash: Default::default(),
        }
    }

    fn parse_relative(&self) -> Url {
        let (path, query) = match self.0.split_once('?') {
            Some((path, query)) => (path, query),
            None => (&*self.0, ""),
        };

        // Normalize a missing leading slash, matching the previous `url::Url`
        // behavior where `foo/bar` became `/foo/bar` and an empty path `/`. A
        // leading `//` is kept literally in the path rather than reinterpreted
        // as a protocol-relative authority, so injected input cannot reach
        // `origin`.
        let path = if path.is_empty() {
            String::from("/")
        } else if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{path}")
        };

        // Decode each query pair exactly once, mirroring the previous
        // `url::Url::query_pairs` behavior (application/x-www-form-urlencoded:
        // `+` -> space, then percent-decode). `insert_decoded` stores the
        // already-decoded values as-is to avoid a second decode.
        let mut search_params = ParamsMap::new();
        for (k, v) in url::form_urlencoded::parse(query.as_bytes()) {
            search_params.insert_decoded(k.into_owned(), v.into_owned());
        }

        Url {
            origin: String::new(),
            path,
            search: query.to_string(),
            search_params,
            hash: Default::default(),
        }
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

    #[test]
    pub fn should_keep_protocol_relative_path_literal() {
        // A leading `//` must stay in the path and must NOT be reinterpreted as
        // a protocol-relative authority, which would otherwise let injected
        // input set `origin` (e.g. host `evil.com`).
        let url = RequestUrl::new("//evil.com/foo").parse().unwrap();
        assert_eq!(url.path(), "//evil.com/foo");
        assert_eq!(url.origin(), "");
    }

    #[test]
    pub fn should_default_empty_path_to_root() {
        let url = RequestUrl::new("").parse().unwrap();
        assert_eq!(url.path(), "/");
    }

    #[test]
    pub fn should_leave_relative_origin_empty() {
        let url = RequestUrl::new("/foo?bar=baz").parse().unwrap();
        assert_eq!(url.origin(), "");
        assert_eq!(url.path(), "/foo");
        assert_eq!(url.search(), "bar=baz");
    }

    #[test]
    pub fn should_decode_plus_as_space_in_query() {
        // `application/x-www-form-urlencoded` decodes `+` to a space, matching
        // the previous `url::Url::query_pairs` behavior. The raw `search`
        // string is preserved verbatim.
        let url = RequestUrl::new("/?q=a+b").parse().unwrap();
        assert_eq!(url.search_params().get("q").unwrap(), "a b");
        assert_eq!(url.search(), "q=a+b");
    }

    #[test]
    pub fn should_extract_real_origin_from_absolute_request_url() {
        // The integrations prefix the real origin, e.g. `{scheme}://{host}{path}`,
        // so `origin()` reflects the actual request host (and matches the
        // client after hydration), with path/query decomposed as before.
        let url = RequestUrl::new("http://127.0.0.1:3000/foo/bar?q=1&r=2")
            .parse()
            .unwrap();
        assert_eq!(url.origin(), "http://127.0.0.1:3000");
        assert_eq!(url.path(), "/foo/bar");
        assert_eq!(url.search(), "q=1&r=2");
        assert_eq!(url.search_params().get("q").unwrap(), "1");
        assert_eq!(url.search_params().get("r").unwrap(), "2");
    }

    #[test]
    pub fn absolute_request_url_keeps_real_origin_for_protocol_relative_path() {
        // A `//`-prefixed path carried after the real origin must stay in the
        // path and must NOT override the authority, so injected input cannot
        // change `origin`.
        let url = RequestUrl::new("http://127.0.0.1:3000//evil.com/foo")
            .parse()
            .unwrap();
        assert_eq!(url.origin(), "http://127.0.0.1:3000");
        assert_eq!(url.path(), "//evil.com/foo");
    }
}
