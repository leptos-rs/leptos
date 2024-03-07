use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSegment {
    Unit,
    Static(Cow<'static, str>),
    Param(Cow<'static, str>),
    Splat(Cow<'static, str>),
}
