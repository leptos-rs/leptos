use std::borrow::Cow;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Params(Vec<(Cow<'static, str>, String)>);

impl Params {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<K, V> FromIterator<(K, V)> for Params
where
    K: Into<Cow<'static, str>>,
    V: Into<String>,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        Self(
            iter.into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }
}
