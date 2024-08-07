use super::{
    Mountable, Position, PositionState, Render, RenderHtml, ToTemplate,
};
use crate::{
    hydration::Cursor,
    no_attrs,
    renderer::{CastFrom, Renderer},
};
use std::{borrow::Cow, rc::Rc, sync::Arc};

no_attrs!(&'a str);
no_attrs!(String);
no_attrs!(Arc<str>);
no_attrs!(Cow<'a, str>);

/// Retained view state for `&str`.
pub struct StrState<'a, R: Renderer> {
    pub(crate) node: R::Text,
    str: &'a str,
}

impl<'a, R: Renderer> Render<R> for &'a str {
    type State = StrState<'a, R>;

    fn build(self) -> Self::State {
        let node = R::create_text_node(self);
        StrState { node, str: self }
    }

    fn rebuild(self, state: &mut Self::State) {
        let StrState { node, str } = state;
        if &self != str {
            R::set_text(node, self);
            *str = self;
        }
    }
}

impl<'a, R> RenderHtml<R> for &'a str
where
    R: Renderer,
{
    type AsyncOutput = Self;

    const MIN_LENGTH: usize = 0;

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }

    fn html_len(&self) -> usize {
        self.len()
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        _mark_branches: bool,
    ) {
        // add a comment node to separate from previous sibling, if any
        if matches!(position, Position::NextChildAfterText) {
            buf.push_str("<!>")
        }
        if self.is_empty() {
            buf.push(' ');
        } else if escape {
            let escaped = html_escape::encode_text(self);
            buf.push_str(&escaped);
        } else {
            buf.push_str(self);
        }
        *position = Position::NextChildAfterText;
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State {
        if position.get() == Position::FirstChild {
            cursor.child();
        } else {
            cursor.sibling();
        }

        // separating placeholder marker comes before text node
        if matches!(position.get(), Position::NextChildAfterText) {
            cursor.sibling();
        }

        let node = cursor.current();
        let node = R::Text::cast_from(node)
            .expect("couldn't cast text node from node");

        if !FROM_SERVER {
            R::set_text(&node, self);
        }
        position.set(Position::NextChildAfterText);

        StrState { node, str: self }
    }
}

impl<'a> ToTemplate for &'a str {
    const TEMPLATE: &'static str = " <!>";

    fn to_template(
        buf: &mut String,
        _class: &mut String,
        _style: &mut String,
        _inner_html: &mut String,
        position: &mut Position,
    ) {
        if matches!(*position, Position::NextChildAfterText) {
            buf.push_str("<!>")
        }
        buf.push(' ');
        *position = Position::NextChildAfterText;
    }
}

impl<'a, R> Mountable<R> for StrState<'a, R>
where
    R: Renderer,
{
    fn unmount(&mut self) {
        self.node.unmount()
    }

    fn mount(
        &mut self,
        parent: &<R as Renderer>::Element,
        marker: Option<&<R as Renderer>::Node>,
    ) {
        R::insert_node(parent, self.node.as_ref(), marker);
    }

    fn insert_before_this(&self, child: &mut dyn Mountable<R>) -> bool {
        self.node.insert_before_this(child)
    }
}

/// Retained view state for `String`.
pub struct StringState<R: Renderer> {
    node: R::Text,
    str: String,
}

impl<R: Renderer> Render<R> for String {
    type State = StringState<R>;

    fn build(self) -> Self::State {
        let node = R::create_text_node(&self);
        StringState { node, str: self }
    }

    fn rebuild(self, state: &mut Self::State) {
        let StringState { node, str } = state;
        if &self != str {
            R::set_text(node, &self);
            *str = self;
        }
    }
}

impl<R> RenderHtml<R> for String
where
    R: Renderer,
{
    const MIN_LENGTH: usize = 0;
    type AsyncOutput = Self;

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }

    fn html_len(&self) -> usize {
        self.len()
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
    ) {
        <&str as RenderHtml<R>>::to_html_with_buf(
            self.as_str(),
            buf,
            position,
            escape,
            mark_branches,
        )
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State {
        let StrState { node, .. } =
            self.as_str().hydrate::<FROM_SERVER>(cursor, position);
        StringState { node, str: self }
    }
}

impl ToTemplate for String {
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

impl<R: Renderer> Mountable<R> for StringState<R> {
    fn unmount(&mut self) {
        self.node.unmount()
    }

    fn mount(
        &mut self,
        parent: &<R as Renderer>::Element,
        marker: Option<&<R as Renderer>::Node>,
    ) {
        R::insert_node(parent, self.node.as_ref(), marker);
    }

    fn insert_before_this(&self, child: &mut dyn Mountable<R>) -> bool {
        self.node.insert_before_this(child)
    }
}

/// Retained view state for `Rc<str>`.
pub struct RcStrState<R: Renderer> {
    node: R::Text,
    str: Rc<str>,
}

impl<R: Renderer> Render<R> for Rc<str> {
    type State = RcStrState<R>;

    fn build(self) -> Self::State {
        let node = R::create_text_node(&self);
        RcStrState { node, str: self }
    }

    fn rebuild(self, state: &mut Self::State) {
        let RcStrState { node, str } = state;
        if !Rc::ptr_eq(&self, str) {
            R::set_text(node, &self);
            *str = self;
        }
    }
}

// can't Send an Rc<str> between threads, so can't implement async HTML rendering that might need
// to send it
/*
impl<R> RenderHtml<R> for Rc<str>
where
    R: Renderer,
{
    type AsyncOutput = Self;

    const MIN_LENGTH: usize = 0;

    async fn resolve(self) -> Self::AsyncOutput {
    self
    }

    fn html_len(&self) -> usize {
        self.len()
    }

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position, escape: bool, mark_branches: bool) {
        <&str as RenderHtml<R>>::to_html_with_buf(&self, buf, position)
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State {
        let this: &str = self.as_ref();
        let StrState { node, .. } =
            this.hydrate::<FROM_SERVER>(cursor, position);
        RcStrState { node, str: self }
    }
}*/

impl ToTemplate for Rc<str> {
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

impl<R: Renderer> Mountable<R> for RcStrState<R> {
    fn unmount(&mut self) {
        self.node.unmount()
    }

    fn mount(
        &mut self,
        parent: &<R as Renderer>::Element,
        marker: Option<&<R as Renderer>::Node>,
    ) {
        R::insert_node(parent, self.node.as_ref(), marker);
    }

    fn insert_before_this(&self, child: &mut dyn Mountable<R>) -> bool {
        self.node.insert_before_this(child)
    }
}

/// Retained view state for `Arc<str>`.
pub struct ArcStrState<R: Renderer> {
    node: R::Text,
    str: Arc<str>,
}

impl<R: Renderer> Render<R> for Arc<str> {
    type State = ArcStrState<R>;

    fn build(self) -> Self::State {
        let node = R::create_text_node(&self);
        ArcStrState { node, str: self }
    }

    fn rebuild(self, state: &mut Self::State) {
        let ArcStrState { node, str } = state;
        if !Arc::ptr_eq(&self, str) {
            R::set_text(node, &self);
            *str = self;
        }
    }
}

impl<R> RenderHtml<R> for Arc<str>
where
    R: Renderer,
{
    type AsyncOutput = Self;

    const MIN_LENGTH: usize = 0;

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }

    fn html_len(&self) -> usize {
        self.len()
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
    ) {
        <&str as RenderHtml<R>>::to_html_with_buf(
            &self,
            buf,
            position,
            escape,
            mark_branches,
        )
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State {
        let this: &str = self.as_ref();
        let StrState { node, .. } =
            this.hydrate::<FROM_SERVER>(cursor, position);
        ArcStrState { node, str: self }
    }
}

impl ToTemplate for Arc<str> {
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

impl<R: Renderer> Mountable<R> for ArcStrState<R> {
    fn unmount(&mut self) {
        self.node.unmount()
    }

    fn mount(
        &mut self,
        parent: &<R as Renderer>::Element,
        marker: Option<&<R as Renderer>::Node>,
    ) {
        R::insert_node(parent, self.node.as_ref(), marker);
    }

    fn insert_before_this(&self, child: &mut dyn Mountable<R>) -> bool {
        self.node.insert_before_this(child)
    }
}

/// Retained view state for `Cow<'_, str>`.
pub struct CowStrState<'a, R: Renderer> {
    node: R::Text,
    str: Cow<'a, str>,
}

impl<'a, R: Renderer> Render<R> for Cow<'a, str> {
    type State = CowStrState<'a, R>;

    fn build(self) -> Self::State {
        let node = R::create_text_node(&self);
        CowStrState { node, str: self }
    }

    fn rebuild(self, state: &mut Self::State) {
        let CowStrState { node, str } = state;
        if self != *str {
            R::set_text(node, &self);
            *str = self;
        }
    }
}

impl<'a, R> RenderHtml<R> for Cow<'a, str>
where
    R: Renderer,
{
    type AsyncOutput = Self;

    const MIN_LENGTH: usize = 0;

    fn dry_resolve(&mut self) {}

    async fn resolve(self) -> Self::AsyncOutput {
        self
    }

    fn html_len(&self) -> usize {
        self.len()
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
    ) {
        <&str as RenderHtml<R>>::to_html_with_buf(
            &self,
            buf,
            position,
            escape,
            mark_branches,
        )
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<R>,
        position: &PositionState,
    ) -> Self::State {
        let this: &str = self.as_ref();
        let StrState { node, .. } =
            this.hydrate::<FROM_SERVER>(cursor, position);
        CowStrState { node, str: self }
    }
}

impl<'a> ToTemplate for Cow<'a, str> {
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

impl<'a, R: Renderer> Mountable<R> for CowStrState<'a, R> {
    fn unmount(&mut self) {
        self.node.unmount()
    }

    fn mount(
        &mut self,
        parent: &<R as Renderer>::Element,
        marker: Option<&<R as Renderer>::Node>,
    ) {
        R::insert_node(parent, self.node.as_ref(), marker);
    }

    fn insert_before_this(&self, child: &mut dyn Mountable<R>) -> bool {
        self.node.insert_before_this(child)
    }
}
