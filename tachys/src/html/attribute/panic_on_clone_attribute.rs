use super::{Attribute, NextAttribute};

/// When type erasing with `AnyAttribute`, the underling attribute must be cloneable.
///
/// For most this is possible, but for some like `NodeRef` it is not.
///
/// This allows for a panic to be thrown if a non-cloneable attribute is cloned, whilst still seeming like it can be cloned.
pub struct PanicOnCloneAttr<T: Attribute + 'static> {
    msg: &'static str,
    attr: T,
}

impl<T: Attribute + 'static> PanicOnCloneAttr<T> {
    pub(crate) fn new(attr: T, msg: &'static str) -> Self {
        Self { msg, attr }
    }
}

impl<T: Attribute + 'static> Clone for PanicOnCloneAttr<T> {
    fn clone(&self) -> Self {
        panic!("{}", self.msg)
    }
}

impl<T: Attribute + 'static> NextAttribute for PanicOnCloneAttr<T> {
    type Output<NewAttr: Attribute> = <T as NextAttribute>::Output<NewAttr>;

    fn add_any_attr<NewAttr: Attribute>(
        self,
        new_attr: NewAttr,
    ) -> Self::Output<NewAttr> {
        self.attr.add_any_attr(new_attr)
    }
}

impl<T: Attribute + 'static> Attribute for PanicOnCloneAttr<T> {
    const MIN_LENGTH: usize = T::MIN_LENGTH;

    type State = T::State;
    type AsyncOutput = T::AsyncOutput;
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn html_len(&self) -> usize {
        self.attr.html_len()
    }

    fn to_html(
        self,
        buf: &mut String,
        class: &mut String,
        style: &mut String,
        inner_html: &mut String,
    ) {
        self.attr.to_html(buf, class, style, inner_html)
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        self.attr.hydrate::<FROM_SERVER>(el)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        self.attr.build(el)
    }

    fn rebuild(self, state: &mut Self::State) {
        self.attr.rebuild(state)
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self
    }

    fn dry_resolve(&mut self) {
        self.attr.dry_resolve()
    }

    async fn resolve(self) -> Self::AsyncOutput {
        self.attr.resolve().await
    }
}
