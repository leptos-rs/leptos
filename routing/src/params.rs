#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Params(Vec<(String, String)>);

impl Params {
    pub fn new() -> Self {
        Self::default()
    }
}

impl FromIterator<(String, String)> for Params {
    fn from_iter<T: IntoIterator<Item = (String, String)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}
