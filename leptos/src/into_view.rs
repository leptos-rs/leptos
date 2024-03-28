use leptos_dom::events::{on, EventDescriptor, On};
use tachys::{
    html::attribute::{global::OnAttribute, Attribute},
    hydration::Cursor,
    renderer::{dom::Dom, DomRenderer, Renderer},
    ssr::StreamBuilder,
    view::{
        add_attr::AddAnyAttr, Mountable, Position, PositionState, Render,
        RenderHtml,
    },
};

pub struct View<T>(T)
where
    T: Sized;

pub trait IntoView: Sized + Render<Dom> + RenderHtml<Dom> + Send
//+ AddAnyAttr<Dom>
{
    fn into_view(self) -> View<Self>;
}

impl<T> IntoView for T
where
    T: Sized + Render<Dom> + RenderHtml<Dom> + Send, //+ AddAnyAttr<Dom>,
{
    fn into_view(self) -> View<Self> {
        View(self)
    }
}

impl<T: Render<Dom>> Render<Dom> for View<T> {
    type State = T::State;
    type FallibleState = T::FallibleState;

    fn build(self) -> Self::State {
        self.0.build()
    }

    fn rebuild(self, state: &mut Self::State) {
        self.0.rebuild(state)
    }

    fn try_build(self) -> any_error::Result<Self::FallibleState> {
        self.0.try_build()
    }

    fn try_rebuild(
        self,
        state: &mut Self::FallibleState,
    ) -> any_error::Result<()> {
        self.0.try_rebuild(state)
    }
}

impl<T: RenderHtml<Dom>> RenderHtml<Dom> for View<T> {
    const MIN_LENGTH: usize = <T as RenderHtml<Dom>>::MIN_LENGTH;

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        self.0.to_html_with_buf(buf, position);
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
    ) where
        Self: Sized,
    {
        self.0.to_html_async_with_buf::<OUT_OF_ORDER>(buf, position)
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<Dom>,
        position: &PositionState,
    ) -> Self::State {
        self.0.hydrate::<FROM_SERVER>(cursor, position)
    }
}

/*impl<T: AddAnyAttr<Dom>> AddAnyAttr<Dom> for View<T> {
    type Output<SomeNewAttr: Attribute<Dom>> =
        <T as AddAnyAttr<Dom>>::Output<SomeNewAttr>;

    fn add_any_attr<NewAttr: Attribute<Dom>>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<Dom>,
    {
        self.0.add_any_attr(attr)
    }

    fn add_any_attr_by_ref<NewAttr: Attribute<Dom>>(
        self,
        attr: &NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<Dom>,
    {
        self.0.add_any_attr_by_ref(attr)
    }
}*/
