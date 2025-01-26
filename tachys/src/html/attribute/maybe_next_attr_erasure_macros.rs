macro_rules! next_attr_output_type {
    ($current:ty, $next:ty) => {
        #[cfg(not(erase_components))]
        type Output<NewAttr: Attribute> = ($current, $next);

        #[cfg(erase_components)]
        type Output<NewAttr: Attribute> =
            Vec<$crate::html::attribute::any_attribute::AnyAttribute>;
    };
}

macro_rules! next_attr_combine {
    ($self:expr, $next_attr:expr) => {{
        #[cfg(not(erase_components))]
        {
            ($self, $next_attr)
        }
        #[cfg(erase_components)]
        {
            use $crate::html::attribute::any_attribute::IntoAnyAttribute;
            vec![$self.into_any_attr(), $next_attr.into_any_attr()]
        }
    }};
}

pub(crate) use next_attr_combine;
pub(crate) use next_attr_output_type;
