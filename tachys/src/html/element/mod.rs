#[cfg(any(debug_assertions, leptos_debuginfo))]
use crate::hydration::set_currently_hydrating;
#[cfg(erase_components)]
use crate::view::any_view::AnyView;
use crate::{
    html::attribute::Attribute,
    hydration::{failed_to_cast_element, Cursor},
    renderer::{CastFrom, Rndr},
    ssr::StreamBuilder,
    view::{
        add_attr::AddAnyAttr, IntoRender, Mountable, Position, PositionState,
        Render, RenderHtml, ToTemplate,
    },
};
use const_str_slice_concat::{
    const_concat, const_concat_with_prefix, str_from_buffer,
};
use futures::future::join;
use std::ops::Deref;

mod custom;
mod element_ext;
mod elements;
mod inner_html;
use super::attribute::{
    any_attribute::AnyAttribute, escape_attr, NextAttribute,
};
pub use custom::*;
pub use element_ext::*;
pub use elements::*;
pub use inner_html::*;
#[cfg(any(debug_assertions, leptos_debuginfo))]
use std::panic::Location;

/// The typed representation of an HTML element.
#[derive(Debug, PartialEq, Eq)]
pub struct HtmlElement<E, At, Ch> {
    #[cfg(any(debug_assertions, leptos_debuginfo))]
    pub(crate) defined_at: &'static Location<'static>,
    pub(crate) tag: E,
    pub(crate) attributes: At,
    pub(crate) children: Ch,
}

impl<E: Clone, At: Clone, Ch: Clone> Clone for HtmlElement<E, At, Ch> {
    fn clone(&self) -> Self {
        HtmlElement {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: self.defined_at,
            tag: self.tag.clone(),
            attributes: self.attributes.clone(),
            children: self.children.clone(),
        }
    }
}

impl<E: Copy, At: Copy, Ch: Copy> Copy for HtmlElement<E, At, Ch> {}

/*impl<E, At, Ch> ElementType for HtmlElement<E, At, Ch>
where
    E: ElementType,
{
    type Output = E::Output;

    const TAG: &'static str = E::TAG;

    const SELF_CLOSING: bool = E::SELF_CLOSING;

    fn tag(&self) -> &str {
        Self::TAG
    }
}*/

#[cfg(not(erase_components))]
impl<E, At, Ch, NewChild> ElementChild<NewChild> for HtmlElement<E, At, Ch>
where
    E: ElementWithChildren,
    Ch: RenderHtml + next_tuple::NextTuple,
    <Ch as next_tuple::NextTuple>::Output<NewChild::Output>: Render,

    NewChild: IntoRender,
    NewChild::Output: RenderHtml,
{
    type Output = HtmlElement<
        E,
        At,
        <Ch as next_tuple::NextTuple>::Output<NewChild::Output>,
    >;

    fn child(self, child: NewChild) -> Self::Output {
        HtmlElement {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: self.defined_at,
            tag: self.tag,
            attributes: self.attributes,
            children: self.children.next_tuple(child.into_render()),
        }
    }
}

#[cfg(erase_components)]
impl<E, At, Ch, NewChild> ElementChild<NewChild> for HtmlElement<E, At, Ch>
where
    E: ElementWithChildren,
    Ch: RenderHtml + NextChildren,

    NewChild: IntoRender,
    NewChild::Output: RenderHtml,
{
    type Output =
        HtmlElement<E, At, crate::view::iterators::StaticVec<AnyView>>;

    fn child(self, child: NewChild) -> Self::Output {
        use crate::view::any_view::IntoAny;

        HtmlElement {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: self.defined_at,
            tag: self.tag,
            attributes: self.attributes,
            children: self
                .children
                .next_children(child.into_render().into_any()),
        }
    }
}

#[cfg(erase_components)]
trait NextChildren {
    fn next_children(
        self,
        child: AnyView,
    ) -> crate::view::iterators::StaticVec<AnyView>;
}

#[cfg(erase_components)]
mod erased_tuples {
    use super::*;
    use crate::view::{any_view::IntoAny, iterators::StaticVec};

    impl NextChildren for StaticVec<AnyView> {
        fn next_children(mut self, child: AnyView) -> StaticVec<AnyView> {
            self.0.push(child);
            self
        }
    }

    impl NextChildren for () {
        fn next_children(self, child: AnyView) -> StaticVec<AnyView> {
            vec![child].into()
        }
    }

    impl<T: RenderHtml> NextChildren for (T,) {
        fn next_children(self, child: AnyView) -> StaticVec<AnyView> {
            vec![self.0.into_owned().into_any(), child].into()
        }
    }

    macro_rules! impl_next_children_tuples {
        ($($ty:ident),*) => {
            impl<$($ty: RenderHtml),*> NextChildren for ($($ty,)*)
             {
                fn next_children(
                    self, child: AnyView,
                ) -> StaticVec<AnyView> {
                    #[allow(non_snake_case)]
                    let ($($ty,)*) = self;
                    vec![$($ty.into_owned().into_any(),)* child].into()
                }
            }
        };
    }

    impl_next_children_tuples!(AA, BB);
    impl_next_children_tuples!(AA, BB, CC);
    impl_next_children_tuples!(AA, BB, CC, DD);
    impl_next_children_tuples!(AA, BB, CC, DD, EE);
    impl_next_children_tuples!(AA, BB, CC, DD, EE, FF);
    impl_next_children_tuples!(AA, BB, CC, DD, EE, FF, GG);
    impl_next_children_tuples!(AA, BB, CC, DD, EE, FF, GG, HH);
    impl_next_children_tuples!(AA, BB, CC, DD, EE, FF, GG, HH, II);
    impl_next_children_tuples!(AA, BB, CC, DD, EE, FF, GG, HH, II, JJ);
    impl_next_children_tuples!(AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK);
    impl_next_children_tuples!(AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK, LL);
    impl_next_children_tuples!(
        AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK, LL, MM
    );
    impl_next_children_tuples!(
        AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK, LL, MM, NN
    );
    impl_next_children_tuples!(
        AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK, LL, MM, NN, OO
    );
    impl_next_children_tuples!(
        AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK, LL, MM, NN, OO, PP
    );
    impl_next_children_tuples!(
        AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK, LL, MM, NN, OO, PP, QQ
    );
    impl_next_children_tuples!(
        AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK, LL, MM, NN, OO, PP, QQ, RR
    );
    impl_next_children_tuples!(
        AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK, LL, MM, NN, OO, PP, QQ, RR,
        SS
    );
    impl_next_children_tuples!(
        AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK, LL, MM, NN, OO, PP, QQ, RR,
        SS, TT
    );
    impl_next_children_tuples!(
        AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK, LL, MM, NN, OO, PP, QQ, RR,
        SS, TT, UU
    );
    impl_next_children_tuples!(
        AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK, LL, MM, NN, OO, PP, QQ, RR,
        SS, TT, UU, VV
    );
    impl_next_children_tuples!(
        AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK, LL, MM, NN, OO, PP, QQ, RR,
        SS, TT, UU, VV, WW
    );
    impl_next_children_tuples!(
        AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK, LL, MM, NN, OO, PP, QQ, RR,
        SS, TT, UU, VV, WW, XX
    );
    impl_next_children_tuples!(
        AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK, LL, MM, NN, OO, PP, QQ, RR,
        SS, TT, UU, VV, WW, XX, YY
    );
}

impl<E, At, Ch> AddAnyAttr for HtmlElement<E, At, Ch>
where
    E: ElementType + Send,
    At: Attribute + Send,
    Ch: RenderHtml + Send,
{
    type Output<SomeNewAttr: Attribute> =
        HtmlElement<E, <At as NextAttribute>::Output<SomeNewAttr>, Ch>;

    fn add_any_attr<NewAttr: Attribute>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr> {
        let HtmlElement {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at,
            tag,
            attributes,
            children,
        } = self;
        HtmlElement {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at,
            tag,
            attributes: attributes.add_any_attr(attr),
            children,
        }
    }
}

/// Adds a child to the element.
pub trait ElementChild<NewChild>
where
    NewChild: IntoRender,
{
    /// The type of the element, with the child added.
    type Output;

    /// Adds a child to an element.
    fn child(self, child: NewChild) -> Self::Output;
}

/// An HTML element.
pub trait ElementType: Send + 'static {
    /// The underlying native widget type that this represents.
    type Output;

    /// The element's tag.
    const TAG: &'static str;
    /// Whether the element is self-closing.
    const SELF_CLOSING: bool;
    /// Whether the element's children should be escaped. This should be `true` except for elements
    /// like `<style>` and `<script>`, which include other languages that should not use HTML
    /// entity escaping.
    const ESCAPE_CHILDREN: bool;
    /// The element's namespace, if it is not HTML.
    const NAMESPACE: Option<&'static str>;

    /// The element's tag.
    fn tag(&self) -> &str;
}

/// Denotes that the type that implements this has a particular HTML element type.
pub trait HasElementType {
    /// The element type.
    type ElementType;
}

pub(crate) trait ElementWithChildren {}

impl<E, At, Ch> HasElementType for HtmlElement<E, At, Ch>
where
    E: ElementType,
{
    type ElementType = E::Output;
}

impl<E, At, Ch> Render for HtmlElement<E, At, Ch>
where
    E: ElementType,
    At: Attribute,
    Ch: Render,
{
    type State = ElementState<At::State, Ch::State>;

    fn rebuild(self, state: &mut Self::State) {
        let ElementState {
            attrs, children, ..
        } = state;
        self.attributes.rebuild(attrs);
        if let Some(children) = children {
            self.children.rebuild(children);
        }
    }

    fn build(self) -> Self::State {
        let el = Rndr::create_element(self.tag.tag(), E::NAMESPACE);

        let attrs = self.attributes.build(&el);
        let children = if E::SELF_CLOSING {
            None
        } else {
            let mut children = self.children.build();
            children.mount(&el, None);
            Some(children)
        };
        ElementState {
            el,
            attrs,
            children,
        }
    }
}

impl<E, At, Ch> RenderHtml for HtmlElement<E, At, Ch>
where
    E: ElementType + Send,
    At: Attribute + Send,
    Ch: RenderHtml + Send,
{
    type AsyncOutput = HtmlElement<E, At::AsyncOutput, Ch::AsyncOutput>;
    type Owned = HtmlElement<E, At::CloneableOwned, Ch::Owned>;

    const MIN_LENGTH: usize = if E::SELF_CLOSING {
        3 // < ... />
        + E::TAG.len()
        + At::MIN_LENGTH
    } else {
        2 // < ... >
        + E::TAG.len()
        + At::MIN_LENGTH
        + Ch::MIN_LENGTH
        + 3 // </ ... >
        + E::TAG.len()
    };

    fn dry_resolve(&mut self) {
        self.attributes.dry_resolve();
        self.children.dry_resolve();
    }

    async fn resolve(self) -> Self::AsyncOutput {
        let (attributes, children) =
            join(self.attributes.resolve(), self.children.resolve()).await;
        HtmlElement {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: self.defined_at,
            tag: self.tag,
            attributes,
            children,
        }
    }

    fn html_len(&self) -> usize {
        if E::SELF_CLOSING {
            3 // < ... />
        + E::TAG.len()
        + self.attributes.html_len()
        } else {
            2 // < ... >
        + E::TAG.len()
        + self.attributes.html_len()
        + self.children.html_len()
        + 3 // </ ... >
        + E::TAG.len()
        }
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        _escape: bool,
        mark_branches: bool,
        extra_attributes: Vec<AnyAttribute>,
    ) {
        // opening tag
        buf.push('<');
        buf.push_str(self.tag.tag());

        let inner_html =
            attributes_to_html((self.attributes, extra_attributes), buf);

        buf.push('>');

        if !E::SELF_CLOSING {
            if !inner_html.is_empty() {
                buf.push_str(&inner_html);
            } else if Ch::EXISTS {
                // children
                *position = Position::FirstChild;
                self.children.to_html_with_buf(
                    buf,
                    position,
                    E::ESCAPE_CHILDREN,
                    mark_branches,
                    vec![],
                );
            }

            // closing tag
            buf.push_str("</");
            buf.push_str(self.tag.tag());
            buf.push('>');
        }
        *position = Position::NextChild;
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buffer: &mut StreamBuilder,
        position: &mut Position,
        _escape: bool,
        mark_branches: bool,
        extra_attributes: Vec<AnyAttribute>,
    ) where
        Self: Sized,
    {
        let mut buf = String::with_capacity(Self::MIN_LENGTH);
        // opening tag
        buf.push('<');
        buf.push_str(self.tag.tag());

        let inner_html =
            attributes_to_html((self.attributes, extra_attributes), &mut buf);

        buf.push('>');
        buffer.push_sync(&buf);

        if !E::SELF_CLOSING {
            // children
            *position = Position::FirstChild;
            if !inner_html.is_empty() {
                buffer.push_sync(&inner_html);
            } else if Ch::EXISTS {
                self.children.to_html_async_with_buf::<OUT_OF_ORDER>(
                    buffer,
                    position,
                    E::ESCAPE_CHILDREN,
                    mark_branches,
                    vec![],
                );
            }

            // closing tag
            let mut buf = String::with_capacity(3 + E::TAG.len());
            buf.push_str("</");
            buf.push_str(self.tag.tag());
            buf.push('>');
            buffer.push_sync(&buf);
        }
        *position = Position::NextChild;
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor,
        position: &PositionState,
    ) -> Self::State {
        // non-Static custom elements need special support in templates
        // because they haven't been inserted type-wise
        if E::TAG.is_empty() && !FROM_SERVER {
            panic!("Custom elements are not supported in ViewTemplate.");
        }

        // codegen optimisation:
        fn inner_1(
            cursor: &Cursor,
            position: &PositionState,
            tag_name: &str,
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: &'static std::panic::Location<'static>,
        ) -> crate::renderer::types::Element {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            {
                set_currently_hydrating(Some(defined_at));
            }

            let curr_position = position.get();
            if curr_position == Position::FirstChild {
                cursor.child();
            } else if curr_position != Position::Current {
                cursor.sibling();
            }
            crate::renderer::types::Element::cast_from(cursor.current())
                .unwrap_or_else(|| {
                    failed_to_cast_element(tag_name, cursor.current())
                })
        }
        let el = inner_1(
            cursor,
            position,
            E::TAG,
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            self.defined_at,
        );

        let attrs = self.attributes.hydrate::<FROM_SERVER>(&el);

        // hydrate children
        let children = if !Ch::EXISTS || !E::ESCAPE_CHILDREN {
            None
        } else {
            position.set(Position::FirstChild);
            Some(self.children.hydrate::<FROM_SERVER>(cursor, position))
        };

        // codegen optimisation:
        fn inner_2(
            cursor: &Cursor,
            position: &PositionState,
            el: &crate::renderer::types::Element,
        ) {
            // go to next sibling
            cursor.set(
                <crate::renderer::types::Element as AsRef<
                    crate::renderer::types::Node,
                >>::as_ref(el)
                .clone(),
            );
            position.set(Position::NextChild);
        }
        inner_2(cursor, position, &el);

        ElementState {
            el,
            attrs,
            children,
        }
    }

    fn into_owned(self) -> Self::Owned {
        HtmlElement {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: self.defined_at,
            tag: self.tag,
            attributes: self.attributes.into_cloneable_owned(),
            children: self.children.into_owned(),
        }
    }
}

/// Renders an [`Attribute`] (which can be one or more HTML attributes) into an HTML buffer.
pub fn attributes_to_html<At>(attr: At, buf: &mut String) -> String
where
    At: Attribute,
{
    // `class` and `style` are created first, and pushed later
    // this is because they can be filled by a mixture of values that include
    // either the whole value (`class="..."` or `style="..."`) and individual
    // classes and styles (`class:foo=true` or `style:height="40px"`), so they
    // need to be filled during the whole attribute-creation process and then
    // added

    // String doesn't allocate until the first push, so this is cheap if there
    // is no class or style on an element
    let mut class = String::new();
    let mut style = String::new();
    let mut inner_html = String::new();

    // inject regular attributes, and fill class and style
    attr.to_html(buf, &mut class, &mut style, &mut inner_html);

    if !class.is_empty() {
        buf.push(' ');
        buf.push_str("class=\"");
        buf.push_str(&escape_attr(class.trim_start().trim_end()));
        buf.push('"');
    }
    if !style.is_empty() {
        buf.push(' ');
        buf.push_str("style=\"");
        buf.push_str(&escape_attr(style.trim_start().trim_end()));
        buf.push('"');
    }

    inner_html
}

/// The retained view state for an HTML element.
pub struct ElementState<At, Ch> {
    pub(crate) el: crate::renderer::types::Element,
    pub(crate) attrs: At,
    pub(crate) children: Option<Ch>,
}

impl<At, Ch> Deref for ElementState<At, Ch> {
    type Target = crate::renderer::types::Element;

    fn deref(&self) -> &Self::Target {
        &self.el
    }
}

impl<At, Ch> Mountable for ElementState<At, Ch> {
    fn unmount(&mut self) {
        Rndr::remove(self.el.as_ref());
    }

    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        Rndr::insert_node(parent, self.el.as_ref(), marker);
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        if let Some(parent) = Rndr::get_parent(self.el.as_ref()) {
            if let Some(element) =
                crate::renderer::types::Element::cast_from(parent)
            {
                child.mount(&element, Some(self.el.as_ref()));
                return true;
            }
        }
        false
    }

    fn elements(&self) -> Vec<crate::renderer::types::Element> {
        vec![self.el.clone()]
    }
}

impl<E, At, Ch> ToTemplate for HtmlElement<E, At, Ch>
where
    E: ElementType,
    At: Attribute + ToTemplate,
    Ch: Render + ToTemplate,
{
    const TEMPLATE: &'static str = str_from_buffer(&const_concat(&[
        "<",
        E::TAG,
        At::TEMPLATE,
        str_from_buffer(&const_concat_with_prefix(
            &[At::CLASS],
            " class=\"",
            "\"",
        )),
        str_from_buffer(&const_concat_with_prefix(
            &[At::STYLE],
            " style=\"",
            "\"",
        )),
        ">",
        Ch::TEMPLATE,
        "</",
        E::TAG,
        ">",
    ]));

    #[allow(unused)] // the variables `class` and `style` might be used, but only with `nightly` feature
    fn to_template(
        buf: &mut String,
        class: &mut String,
        style: &mut String,
        inner_html: &mut String,
        position: &mut Position,
    ) {
        // for custom elements without type known at compile time, do nothing
        if !E::TAG.is_empty() {
            // opening tag and attributes
            let mut class = String::new();
            let mut style = String::new();
            let mut inner_html = String::new();

            buf.push('<');
            buf.push_str(E::TAG);
            <At as ToTemplate>::to_template(
                buf,
                &mut class,
                &mut style,
                &mut inner_html,
                position,
            );

            if !class.is_empty() {
                buf.push(' ');
                buf.push_str("class=\"");
                buf.push_str(class.trim_start().trim_end());
                buf.push('"');
            }
            if !style.is_empty() {
                buf.push(' ');
                buf.push_str("style=\"");
                buf.push_str(style.trim_start().trim_end());
                buf.push('"');
            }
            buf.push('>');

            // children
            *position = Position::FirstChild;
            class.clear();
            style.clear();
            inner_html.clear();
            Ch::to_template(
                buf,
                &mut class,
                &mut style,
                &mut inner_html,
                position,
            );

            // closing tag
            buf.push_str("</");
            buf.push_str(E::TAG);
            buf.push('>');
            *position = Position::NextChild;
        }
    }
}
/*
#[cfg(all(test, feature = "testing"))]
mod tests {
    #[cfg(all(feature = "nightly", rustc_nightly))]
    use super::RenderHtml;
    use super::{main, p, HtmlElement};
    use crate::{
        html::{
            attribute::global::GlobalAttributes,
            element::{em, ElementChild, Main},
        },
        renderer::mock_dom::MockDom,
        view::Render,
    };

    #[test]
    fn mock_dom_creates_element() {
        let el: HtmlElement<Main, _, _, MockDom> =
            main().child(p().id("test").lang("en").child("Hello, world!"));
        let el = el.build();
        assert_eq!(
            el.el.to_debug_html(),
            "<main><p id=\"test\" lang=\"en\">Hello, world!</p></main>"
        );
    }

    #[test]
    fn mock_dom_creates_element_with_several_children() {
        let el: HtmlElement<Main, _, _, MockDom> = main().child(p().child((
            "Hello, ",
            em().child("beautiful"),
            " world!",
        )));
        let el = el.build();
        assert_eq!(
            el.el.to_debug_html(),
            "<main><p>Hello, <em>beautiful</em> world!</p></main>"
        );
    }

    #[cfg(all(feature = "nightly", rustc_nightly))]
    #[test]
    fn html_render_allocates_appropriate_buffer() {
        use crate::view::static_types::Static;

        let el: HtmlElement<Main, _, _, MockDom> = main().child(p().child((
            Static::<"Hello, ">,
            em().child(Static::<"beautiful">),
            Static::<" world!">,
        )));
        let allocated_len = el.html_len();
        let html = el.to_html();
        assert_eq!(
            html,
            "<main><p>Hello, <em>beautiful</em> world!</p></main>"
        );
        assert_eq!(html.len(), allocated_len);
    }
}
 */
