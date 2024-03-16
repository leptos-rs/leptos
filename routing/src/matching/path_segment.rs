use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSegment {
    Unit,
    Static(Cow<'static, str>),
    Param(Cow<'static, str>),
    Splat(Cow<'static, str>),
}

impl PathSegment {
    pub fn as_raw_str(&self) -> &str {
        match self {
            PathSegment::Unit => "",
            PathSegment::Static(i) => i,
            PathSegment::Param(i) => i,
            PathSegment::Splat(i) => i,
        }
    }
}
