use crate::html::attribute::AttributeKey;

/// `group` attribute used for radio inputs with `bind`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Group;

impl AttributeKey for Group {
    const KEY: &'static str = "group";
}
