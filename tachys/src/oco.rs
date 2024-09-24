use crate::{
    html::{attribute::AttributeValue, class::IntoClass},
    hydration::Cursor,
    no_attrs,
    prelude::{Mountable, Render, RenderHtml},
    renderer::Rndr,
    view::{strings::StrState, Position, PositionState, ToTemplate},
};
use oco_ref::Oco;

/// Retained view state for [`Oco`].
pub struct OcoStrState {
    node: crate::renderer::types::Text,
    str: Oco<'static, str>,
}

impl Render for Oco<'static, str> {
    type State = OcoStrState;

    fn build(self) -> Self::State {
        let node = Rndr::create_text_node(&self);
        OcoStrState { node, str: self }
    }

    fn rebuild(self, state: &mut Self::State) {
        let OcoStrState { node, str } = state;
        if &self == str {
            Rndr::set_text(node, &self);
            *str = self;
        }
    }
}

no_attrs!(Oco<'static, str>);

impl RenderHtml for Oco<'static, str> {
    type AsyncOutput = Self;

    const MIN_LENGTH: usize = 0;

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
    ) {
        <&str as RenderHtml>::to_html_with_buf(
            &self,
            buf,
            position,
            escape,
            mark_branches,
        )
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor,
        position: &PositionState,
    ) -> Self::State {
        let this: &str = self.as_ref();
        let StrState { node, .. } = <&str as RenderHtml>::hydrate::<FROM_SERVER>(
            this, cursor, position,
        );
        OcoStrState { node, str: self }
    }
}

impl ToTemplate for Oco<'static, str> {
    const TEMPLATE: &'static str = <&str as ToTemplate>::TEMPLATE;

    fn to_template(
        buf: &mut String,
        class: &mut String,
        style: &mut String,
        inner_html: &mut String,
        position: &mut Position,
    ) {
        <&str as ToTemplate>::to_template(
            buf, class, style, inner_html, position,
        )
    }
}

impl Mountable for OcoStrState {
    fn unmount(&mut self) {
        self.node.unmount()
    }

    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        Rndr::insert_node(parent, self.node.as_ref(), marker);
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        self.node.insert_before_this(child)
    }
}

impl AttributeValue for Oco<'static, str> {
    type AsyncOutput = Self;
    type State = (crate::renderer::types::Element, Oco<'static, str>);
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn html_len(&self) -> usize {
        self.as_str().len()
    }

    fn to_html(self, key: &str, buf: &mut String) {
        <&str as AttributeValue>::to_html(self.as_str(), key, buf);
    }

    fn to_template(_key: &str, _buf: &mut String) {}

    fn hydrate<const FROM_SERVER: bool>(
        self,
        key: &str,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        let (el, _) = <&str as AttributeValue>::hydrate::<FROM_SERVER>(
            self.as_str(),
            key,
            el,
        );
        (el, self)
    }

    fn build(
        self,
        el: &crate::renderer::types::Element,
        key: &str,
    ) -> Self::State {
        Rndr::set_attribute(el, key, &self);
        (el.clone(), self)
    }

    fn rebuild(self, key: &str, state: &mut Self::State) {
        let (el, prev_value) = state;
        if self != *prev_value {
            Rndr::set_attribute(el, key, &self);
        }
        *prev_value = self;
    }

    fn into_cloneable(mut self) -> Self::Cloneable {
        // ensure it's reference-counted
        self.upgrade_inplace();
        self
    }

    fn into_cloneable_owned(mut self) -> Self::CloneableOwned {
        // ensure it's reference-counted
        self.upgrade_inplace();
        self
    }

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }
}

impl IntoClass for Oco<'static, str> {
    type AsyncOutput = Self;
    type State = (crate::renderer::types::Element, Self);
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn html_len(&self) -> usize {
        self.as_str().len()
    }

    fn to_html(self, class: &mut String) {
        IntoClass::to_html(self.as_str(), class);
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        if !FROM_SERVER {
            Rndr::set_attribute(el, "class", &self);
        }
        (el.clone(), self)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        Rndr::set_attribute(el, "class", &self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (el, prev) = state;
        if self != *prev {
            Rndr::set_attribute(el, "class", &self);
        }
        *prev = self;
    }

    fn into_cloneable(mut self) -> Self::Cloneable {
        // ensure it's reference-counted
        self.upgrade_inplace();
        self
    }

    fn into_cloneable_owned(mut self) -> Self::CloneableOwned {
        // ensure it's reference-counted
        self.upgrade_inplace();
        self
    }

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }
}
