use super::{Url, BASE};
use crate::params::ParamsMap;
use leptos::server::ServerActionError;
use reactive_graph::owner::provide_context;
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

        let search_params = url
            .query_pairs()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<ParamsMap>();

        if let (Some(err), Some(path)) = (
            search_params.get_str("__err"),
            search_params.get_str("__path"),
        ) {
            provide_context(ServerActionError::new(path, err))
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
