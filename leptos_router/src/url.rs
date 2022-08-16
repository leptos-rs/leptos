use crate::Params;

#[derive(Debug, Clone)]
pub struct Url {
    pub path_name: String,
    pub search: String,
    pub hash: String,
}

impl Url {
    pub fn search_params(&self) -> Params {
        todo!()
    }
}

impl TryFrom<&str> for Url {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        todo!()
    }
}
