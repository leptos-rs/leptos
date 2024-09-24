use super::RenderHtml;
use crate::html::attribute::Attribute;

/// Allows adding a new attribute to some type, before it is rendered.
/// This takes place at compile time as part of the builder syntax for creating a statically typed
/// view tree.
///
/// Normally, this is used to add an attribute to an HTML element. But it is required to be
/// implemented for all types that implement [`RenderHtml`], so that attributes can be spread onto
/// other structures like the return type of a component.
pub trait AddAnyAttr {
    /// The new type once the attribute has been added.
    type Output<SomeNewAttr: Attribute>: RenderHtml;

    /// Adds an attribute to the view.
    fn add_any_attr<NewAttr: Attribute>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml;
}

/// Declares that spreading attributes onto a particular type has no effect.
#[macro_export]
macro_rules! no_attrs {
    ($ty_name:ty) => {
        impl<'a> $crate::view::add_attr::AddAnyAttr for $ty_name {
            type Output<SomeNewAttr: $crate::html::attribute::Attribute> =
                $ty_name;

            fn add_any_attr<NewAttr: $crate::html::attribute::Attribute>(
                self,
                _attr: NewAttr,
            ) -> Self::Output<NewAttr> {
                self
            }
        }
    };
}
