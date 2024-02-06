use crate::ParamsMap;
#[cfg(not(feature = "ssr"))]
use js_sys::{try_iter, Array, JsString};
#[cfg(not(feature = "ssr"))]
use wasm_bindgen::JsCast;
#[cfg(not(feature = "ssr"))]
use wasm_bindgen::JsValue;

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
    percent_encoding::utf8_percent_encode(s, percent_encoding::NON_ALPHANUMERIC)
        .to_string()
}

#[cfg(not(feature = "ssr"))]
pub fn escape(s: &str) -> String {
    js_sys::encode_uri(s).as_string().unwrap()
}

impl Url {
    #[cfg(not(feature = "ssr"))]
    pub fn new_with_base(url: &str, base: &str) -> Result<Self, String> {
        let url = web_sys::Url::new_with_base(url, base).map_js_error()?;
        Self::from_web_sys_url(&url)
    }

    #[cfg(not(feature = "ssr"))]
    fn from_web_sys_url(url: &web_sys::Url) -> Result<Self, String> {
        Ok(
            Self {
                origin: url.origin(),
                pathname: url.pathname(),
                search: url
                    .search()
                    .strip_prefix('?')
                    .map(String::from)
                    .unwrap_or_default(),
                search_params:
                    ParamsMap(
                        try_iter(&url.search_params())
                            .map_js_error()?
                            .ok_or(
                                "Failed to use URLSearchParams as an iterator"
                                    .to_string(),
                            )?
                            .map(|value| {
                                let array: Array = value
                                    .map_js_error()?
                                    .dyn_into()
                                    .map_js_error()?;
                                Ok((
                                    array
                                        .get(0)
                                        .dyn_into::<JsString>()
                                        .map_js_error()?
                                        .into(),
                                    array
                                        .get(1)
                                        .dyn_into::<JsString>()
                                        .map_js_error()?
                                        .into(),
                                ))
                            })
                            .collect::<Result<
                                linear_map::LinearMap<String, String>,
                                String,
                            >>()?,
                    ),
                hash: url.hash(),
            },
        )
    }

    #[cfg(feature = "ssr")]
    pub fn new_with_base(url: &str, base: &str) -> Result<Self, String> {
        let base = url::Url::parse(base).map_err(|e| e.to_string())?;
        let url = url::Url::options()
            .base_url(Some(&base))
            .parse(url)
            .map_err(|e| e.to_string())?;
        Self::from_servo_url(&url)
    }

    #[cfg(feature = "ssr")]
    fn from_servo_url(url: &url::Url) -> Result<Self, String> {
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

    pub fn href(&self) -> String {
        let question_mark = if self.search.is_empty() { "" } else { "?" };
        format!(
            "{}{}{}{}{}",
            self.origin, self.pathname, question_mark, self.search, self.hash
        )
    }
}

#[cfg(not(feature = "ssr"))]
impl TryFrom<&str> for Url {
    type Error = String;

    fn try_from(url: &str) -> Result<Self, Self::Error> {
        let url = web_sys::Url::new(url).map_js_error()?;
        Self::from_web_sys_url(&url)
    }
}

#[cfg(feature = "ssr")]
impl TryFrom<&str> for Url {
    type Error = String;

    fn try_from(url: &str) -> Result<Self, Self::Error> {
        let url = url::Url::parse(url).map_err(|e| e.to_string())?;
        Self::from_servo_url(&url)
    }
}

#[cfg(not(feature = "ssr"))]
trait MapJsError<T> {
    fn map_js_error(self) -> Result<T, String>;
}

#[cfg(not(feature = "ssr"))]
impl<T> MapJsError<T> for Result<T, JsValue> {
    fn map_js_error(self) -> Result<T, String> {
        self.map_err(|e| e.as_string().unwrap_or_default())
    }
}
