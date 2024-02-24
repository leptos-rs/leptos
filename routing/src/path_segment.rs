use alloc::borrow::Cow;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSegment {
    Static(Cow<'static, str>),
    Param(Cow<'static, str>),
    Splat(Cow<'static, str>),
}
