use crate::TextProp;

#[derive(Default)]
pub struct AdditionalAttributes(pub Option<Vec<(String, TextProp)>>);

impl<I, T, U> From<I> for AdditionalAttributes
where
    I: IntoIterator<Item = (T, U)>,
    T: Into<String>,
    U: Into<TextProp>,
{
    fn from(value: I) -> Self {
        Self(Some(
            value
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        ))
    }
}
