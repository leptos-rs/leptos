use crate::ParamsMap;
use std::borrow::Cow;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Url {
    pub origin: String,
    pub pathname: String,
    pub search: String,
    pub search_params: ParamsMap,
    pub hash: String,
}

#[cfg(not(feature = "ssr"))]
pub fn unescape(s: &str) -> String {
    js_sys::decode_uri(s).unwrap().into()
}

#[cfg(feature = "ssr")]
pub fn escape(s: &str) -> String {
    urlencoding::encode(s).into()
}

#[cfg(not(feature = "ssr"))]
pub fn escape(s: &str) -> String {
    js_sys::encode_uri(s).as_string().unwrap()
}

impl TryFrom<&str> for Url {
    type Error = String;

    fn try_from(url: &str) -> Result<Self, Self::Error> {
        let url = url::Url::parse(&normalize_wasm_url(url)).map_err(|e| e.to_string())?;
        Ok(Self {
            origin: url.origin().unicode_serialization(),
            pathname: url.path().to_string(),
            search: url.query().unwrap_or_default().to_string(),
            search_params: ParamsMap(
                url.query_pairs()
                    .map(|(key, value)| (key.to_string(), value.to_string()))
                    .collect::<linear_map::LinearMap<String, String>>(),
            ),
            hash: Default::default(),
        })
    }
}

#[cfg(feature = "ssr")]
fn normalize_wasm_url(url: &str) -> Cow<'_, str> {
    Cow::Borrowed(url)
}

#[cfg(not(feature = "ssr"))]
fn normalize_wasm_url(url: &str) -> Cow<'_, str> {
    Cow::Owned(format!("http://leptos{}", url))
}
