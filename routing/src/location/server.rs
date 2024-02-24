use super::{Location, LocationChange, Url};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestUrl(String);

impl RequestUrl {
    /// Creates a server-side request URL from a path, with an optional initial slash.
    pub fn from_path(path: impl AsRef<str>) -> Self {
        let path = path.as_ref().trim_start_matches('/');
        let mut string = String::with_capacity(path.len());
        string.push_str(path);
        Self(string)
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

    fn try_to_url_with_base(&self, base: &str) -> Result<Url, Self::Error> {
        let url = String::with_capacity(self.0.len() + 1 + base);
        let url = url::Url::parse(&self.0)?;
        let search_params = url
            .query_pairs()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Ok(Url {
            origin: url.origin().unicode_serialization(),
            pathname: url.path().to_string(),
            search: url.query().unwrap_or_default().to_string(),
            search_params,
            hash: Default::default(),
        })
    }

    fn set_navigation_hook(&mut self, _cb: impl FnMut(Url) + 'static) {}

    fn navigate(&self, _loc: &LocationChange) {}
}

#[cfg(test)]
mod tests {
    use super::RequestUrl;
    use crate::location::Location;

    pub fn should_parse_url_without_origin() {
        let req = RequestUrl::from_path("/foo/bar");
        let url = req.try_to_url().expect("could not parse URL");
    }
}
