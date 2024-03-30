use self::properties::Connect;
use gtk::{
    ffi::GtkWidget,
    glib::{
        object::{IsA, IsClass, ObjectExt},
        Object, Value,
    },
    prelude::{Cast, WidgetExt},
    Label, Orientation, Widget,
};
use leptos::{
    reactive_graph::effect::RenderEffect,
    tachys::{
        renderer::{CastFrom, Renderer},
        view::{Mountable, Render},
    },
};
use next_tuple::TupleBuilder;
use std::{borrow::Cow, marker::PhantomData};

#[derive(Debug)]
pub struct LeptosGtk;

#[derive(Debug, Clone)]
pub struct Element(pub Widget);

impl Element {
    pub fn remove(&self) {
        self.0.unparent();
    }
}

#[derive(Debug, Clone)]
pub struct Text(pub Element);

impl<T> From<T> for Element
where
    T: Into<Widget>,
{
    fn from(value: T) -> Self {
        Element(value.into())
    }
}

impl Mountable<LeptosGtk> for Element {
    fn unmount(&mut self) {
        self.remove()
    }

    fn mount(
        &mut self,
        parent: &<LeptosGtk as Renderer>::Element,
        marker: Option<&<LeptosGtk as Renderer>::Node>,
    ) {
        self.0
            .insert_before(&parent.0, marker.as_ref().map(|m| &m.0));
    }

    fn insert_before_this(
        &self,
        parent: &<LeptosGtk as Renderer>::Element,
        child: &mut dyn Mountable<LeptosGtk>,
    ) -> bool {
        child.mount(parent, Some(self.as_ref()));
        true
    }
}

impl Mountable<LeptosGtk> for Text {
    fn unmount(&mut self) {
        self.0.remove()
    }

    fn mount(
        &mut self,
        parent: &<LeptosGtk as Renderer>::Element,
        marker: Option<&<LeptosGtk as Renderer>::Node>,
    ) {
        self.0
             .0
            .insert_before(&parent.0, marker.as_ref().map(|m| &m.0));
    }

    fn insert_before_this(
        &self,
        parent: &<LeptosGtk as Renderer>::Element,
        child: &mut dyn Mountable<LeptosGtk>,
    ) -> bool {
        child.mount(parent, Some(self.as_ref()));
        true
    }
}

impl CastFrom<Element> for Element {
    fn cast_from(source: Element) -> Option<Self> {
        Some(source)
    }
}

impl CastFrom<Element> for Text {
    fn cast_from(source: Element) -> Option<Self> {
        source
            .0
            .downcast::<Label>()
            .ok()
            .map(|n| Text(Element::from(n)))
    }
}

impl AsRef<Element> for Element {
    fn as_ref(&self) -> &Element {
        self
    }
}

impl AsRef<Element> for Text {
    fn as_ref(&self) -> &Element {
        &self.0
    }
}

impl Renderer for LeptosGtk {
    type Node = Element;
    type Element = Element;
    type Text = Text;
    type Placeholder = Element;

    fn intern(text: &str) -> &str {
        text
    }

    fn create_text_node(text: &str) -> Self::Text {
        Text(Element::from(Label::new(Some(text))))
    }

    fn create_placeholder() -> Self::Placeholder {
        let label = Label::new(None);
        label.set_visible(false);
        Element::from(label)
    }

    fn set_text(node: &Self::Text, text: &str) {
        let node_as_text = node.0 .0.downcast_ref::<Label>().unwrap();
        node_as_text.set_label(text);
    }

    fn set_attribute(node: &Self::Element, name: &str, value: &str) {
        node.0.set_property(name, value);
    }

    fn remove_attribute(node: &Self::Element, name: &str) {
        node.0.set_property(name, None::<&str>);
    }

    fn insert_node(
        parent: &Self::Element,
        new_child: &Self::Node,
        marker: Option<&Self::Node>,
    ) {
        new_child
            .0
            .insert_before(&parent.0, marker.as_ref().map(|n| &n.0));
    }

    fn remove_node(
        parent: &Self::Element,
        child: &Self::Node,
    ) -> Option<Self::Node> {
        todo!()
    }

    fn remove(node: &Self::Node) {
        todo!()
    }

    fn get_parent(node: &Self::Node) -> Option<Self::Node> {
        node.0.parent().map(Element::from)
    }

    fn first_child(node: &Self::Node) -> Option<Self::Node> {
        todo!()
    }

    fn next_sibling(node: &Self::Node) -> Option<Self::Node> {
        todo!()
    }

    fn log_node(node: &Self::Node) {
        todo!()
    }

    fn clear_children(parent: &Self::Element) {
        todo!()
    }
}

fn root<Chil>(children: Chil) -> (Widget, impl Mountable<LeptosGtk>)
where
    Chil: Render<LeptosGtk>,
{
    let state = r#box()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .child(children)
        .build();
    (state.as_widget().clone(), state)
}

pub trait WidgetClass {
    type Widget: Into<Widget> + IsA<Object> + IsClass;
}

pub struct LGtkWidget<Widg, Props, Chil> {
    widget: PhantomData<Widg>,
    properties: Props,
    children: Chil,
}

impl<Widg, Props, Chil> LGtkWidget<Widg, Props, Chil>
where
    Widg: WidgetClass,
    Chil: TupleBuilder,
{
    pub fn child<T>(
        self,
        child: T,
    ) -> LGtkWidget<Widg, Props, Chil::Output<T>> {
        let LGtkWidget {
            widget,
            properties,
            children,
        } = self;
        LGtkWidget {
            widget,
            properties,
            children: children.next_tuple(child),
        }
    }
}
impl<Widg, Props, Chil> LGtkWidget<Widg, Props, Chil>
where
    Widg: WidgetClass,
    Props: TupleBuilder,
    Chil: Render<LeptosGtk>,
{
    pub fn connect<F>(
        self,
        signal_name: &'static str,
        callback: F,
    ) -> LGtkWidget<Widg, Props::Output<Connect<F>>, Chil>
    where
        F: Fn(&[Value]) -> Option<Value> + Send + Sync + 'static,
    {
        let LGtkWidget {
            widget,
            properties,
            children,
        } = self;
        LGtkWidget {
            widget,
            properties: properties.next_tuple(Connect {
                signal_name,
                callback,
            }),
            children,
        }
    }
}

pub struct LGtkWidgetState<Widg, Props, Chil>
where
    Chil: Render<LeptosGtk>,
    Props: Property,
    Widg: WidgetClass,
{
    ty: PhantomData<Widg>,
    widget: Element,
    properties: Props::State,
    children: Chil::State,
}

impl<Widg, Props, Chil> LGtkWidgetState<Widg, Props, Chil>
where
    Chil: Render<LeptosGtk>,
    Props: Property,
    Widg: WidgetClass,
{
    pub fn as_widget(&self) -> &Widget {
        &self.widget.0
    }
}

impl<Widg, Props, Chil> Render<LeptosGtk> for LGtkWidget<Widg, Props, Chil>
where
    Widg: WidgetClass,
    Props: Property,
    Chil: Render<LeptosGtk>,
{
    type State = LGtkWidgetState<Widg, Props, Chil>;
    type FallibleState = ();

    fn build(self) -> Self::State {
        let widget = Object::new::<Widg::Widget>();
        let widget = Element::from(widget);
        let properties = self.properties.build(&widget);
        let mut children = self.children.build();
        children.mount(&widget, None);
        LGtkWidgetState {
            ty: PhantomData,
            widget,
            properties,
            children,
        }
    }

    fn rebuild(self, state: &mut Self::State) {
        self.properties
            .rebuild(&state.widget, &mut state.properties);
        self.children.rebuild(&mut state.children);
    }

    fn try_build(self) -> any_error::Result<Self::FallibleState> {
        todo!()
    }

    fn try_rebuild(
        self,
        state: &mut Self::FallibleState,
    ) -> any_error::Result<()> {
        todo!()
    }
}

impl<Widg, Props, Chil> Mountable<LeptosGtk>
    for LGtkWidgetState<Widg, Props, Chil>
where
    Widg: WidgetClass,
    Props: Property,
    Chil: Render<LeptosGtk>,
{
    fn unmount(&mut self) {
        self.children.unmount();
        self.widget.remove();
    }

    fn mount(
        &mut self,
        parent: &<LeptosGtk as Renderer>::Element,
        marker: Option<&<LeptosGtk as Renderer>::Node>,
    ) {
        println!("mounting {}", std::any::type_name::<Widg>());
        self.children.mount(&self.widget, None);
        LeptosGtk::insert_node(parent, &self.widget, marker);
    }

    fn insert_before_this(
        &self,
        parent: &<LeptosGtk as Renderer>::Element,
        child: &mut dyn Mountable<LeptosGtk>,
    ) -> bool {
        child.mount(parent, Some(self.widget.as_ref()));
        true
    }
}

pub trait Property {
    type State;

    fn build(self, element: &Element) -> Self::State;

    fn rebuild(self, element: &Element, state: &mut Self::State);
}

impl<T, F> Property for F
where
    T: Property,
    T::State: 'static,
    F: Fn() -> T + 'static,
{
    type State = RenderEffect<T::State>;

    fn build(self, widget: &Element) -> Self::State {
        let widget = widget.clone();
        RenderEffect::new(move |prev| {
            let value = self();
            if let Some(mut prev) = prev {
                value.rebuild(&widget, &mut prev);
                prev
            } else {
                value.build(&widget)
            }
        })
    }

    fn rebuild(self, widget: &Element, state: &mut Self::State) {}
}

pub fn button() -> LGtkWidget<gtk::Button, (), ()> {
    LGtkWidget {
        widget: PhantomData,
        properties: (),
        children: (),
    }
}

pub fn r#box() -> LGtkWidget<gtk::Box, (), ()> {
    LGtkWidget {
        widget: PhantomData,
        properties: (),
        children: (),
    }
}

mod widgets {
    use super::WidgetClass;

    impl WidgetClass for gtk::Button {
        type Widget = Self;
    }

    impl WidgetClass for gtk::Box {
        type Widget = Self;
    }
}

pub mod properties {
    use super::{
        Element, LGtkWidget, LGtkWidgetState, LeptosGtk, Property, WidgetClass,
    };
    use gtk::glib::{object::ObjectExt, Value};
    use leptos::tachys::{renderer::Renderer, view::Render};
    use next_tuple::TupleBuilder;

    pub struct Connect<F>
    where
        F: Fn(&[Value]) -> Option<Value> + Send + Sync + 'static,
    {
        pub signal_name: &'static str,
        pub callback: F,
    }

    impl<F> Property for Connect<F>
    where
        F: Fn(&[Value]) -> Option<Value> + Send + Sync + 'static,
    {
        type State = ();

        fn build(self, element: &Element) -> Self::State {
            element.0.connect(self.signal_name, false, self.callback);
        }

        fn rebuild(self, element: &Element, state: &mut Self::State) {}
    }

    /* examples for macro */
    pub struct Orientation {
        value: gtk::Orientation,
    }

    pub struct OrientationState {
        value: gtk::Orientation,
    }

    impl Property for Orientation {
        type State = OrientationState;

        fn build(self, element: &Element) -> Self::State {
            element.0.set_property("orientation", self.value);
            OrientationState { value: self.value }
        }

        fn rebuild(self, element: &Element, state: &mut Self::State) {
            if self.value != state.value {
                element.0.set_property("orientation", self.value);
                state.value = self.value;
            }
        }
    }

    impl<Widg, Props, Chil> LGtkWidget<Widg, Props, Chil>
    where
        Widg: WidgetClass,
        Props: TupleBuilder,
        Chil: Render<LeptosGtk>,
    {
        pub fn orientation(
            self,
            value: impl Into<gtk::Orientation>,
        ) -> LGtkWidget<Widg, Props::Output<Orientation>, Chil> {
            let LGtkWidget {
                widget,
                properties,
                children,
            } = self;
            LGtkWidget {
                widget,
                properties: properties.next_tuple(Orientation {
                    value: value.into(),
                }),
                children,
            }
        }
    }

    pub struct Spacing {
        value: i32,
    }

    pub struct SpacingState {
        value: i32,
    }

    impl Property for Spacing {
        type State = SpacingState;

        fn build(self, element: &Element) -> Self::State {
            element.0.set_property("spacing", self.value);
            SpacingState { value: self.value }
        }

        fn rebuild(self, element: &Element, state: &mut Self::State) {
            if self.value != state.value {
                element.0.set_property("spacing", self.value);
                state.value = self.value;
            }
        }
    }

    impl<Widg, Props, Chil> LGtkWidget<Widg, Props, Chil>
    where
        Widg: WidgetClass,
        Props: TupleBuilder,
        Chil: Render<LeptosGtk>,
    {
        pub fn spacing(
            self,
            value: impl Into<i32>,
        ) -> LGtkWidget<Widg, Props::Output<Spacing>, Chil> {
            let LGtkWidget {
                widget,
                properties,
                children,
            } = self;
            LGtkWidget {
                widget,
                properties: properties.next_tuple(Spacing {
                    value: value.into(),
                }),
                children,
            }
        }
    }

    /* end examples for properties macro */

    pub struct Label {
        value: String,
    }

    impl Label {
        pub fn new(value: impl Into<String>) -> Self {
            Self {
                value: value.into(),
            }
        }
    }

    pub struct LabelState {
        value: String,
    }

    impl Property for Label {
        type State = LabelState;

        fn build(self, element: &Element) -> Self::State {
            LeptosGtk::set_attribute(element, "label", &self.value);
            LabelState { value: self.value }
        }

        fn rebuild(self, element: &Element, state: &mut Self::State) {
            todo!()
        }
    }

    impl Property for () {
        type State = ();

        fn build(self, _element: &Element) -> Self::State {}

        fn rebuild(self, _element: &Element, _state: &mut Self::State) {}
    }

    macro_rules! tuples {
        ($($ty:ident),* $(,)?) => {
            impl<$($ty,)*> Property for ($($ty,)*)
                where $($ty: Property,)*
            {
                type State = ($($ty::State,)*);

                fn build(self, element: &Element) -> Self::State {
                    #[allow(non_snake_case)]
                    let ($($ty,)*) = self;
                    ($($ty.build(element),)*)
                }

                fn rebuild(self, element: &Element, state: &mut Self::State) {
                    paste::paste! {
                        #[allow(non_snake_case)]
                        let ($($ty,)*) = self;
                        #[allow(non_snake_case)]
                        let ($([<state_ $ty:lower>],)*) = state;
                        $($ty.rebuild(element, [<state_ $ty:lower>]));*
                    }
                }
            }
        }
    }

    tuples!(A);
    tuples!(A, B);
    tuples!(A, B, C);
    tuples!(A, B, C, D);
    tuples!(A, B, C, D, E);
    tuples!(A, B, C, D, E, F);
    tuples!(A, B, C, D, E, F, G);
    tuples!(A, B, C, D, E, F, G, H);
    tuples!(A, B, C, D, E, F, G, H, I);
    tuples!(A, B, C, D, E, F, G, H, I, J);
    tuples!(A, B, C, D, E, F, G, H, I, J, K);
    tuples!(A, B, C, D, E, F, G, H, I, J, K, L);
    tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M);
    tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
    tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
    tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
    tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q);
    tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R);
    tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S);
    tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T);
    tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U);
    tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V);
    tuples!(
        A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W
    );
    tuples!(
        A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X
    );
    tuples!(
        A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X,
        Y
    );
}
