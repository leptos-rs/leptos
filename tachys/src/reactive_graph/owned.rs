use crate::{
    html::attribute::Attribute,
    hydration::Cursor,
    prelude::Mountable,
    ssr::StreamBuilder,
    view::{add_attr::AddAnyAttr, Position, PositionState, Render, RenderHtml},
};
use reactive_graph::{computed::ScopedFuture, owner::Owner};

/// A view wrapper that sets the reactive [`Owner`] to a particular owner whenever it is rendered.
#[derive(Debug, Clone)]
pub struct OwnedView<T> {
    owner: Owner,
    view: T,
}

impl<T> OwnedView<T> {
    /// Wraps a view with the current owner.
    pub fn new(view: T) -> Self {
        let owner = Owner::current().expect("no reactive owner");
        Self { owner, view }
    }

    /// Wraps a view with the given owner.
    pub fn new_with_owner(view: T, owner: Owner) -> Self {
        Self { owner, view }
    }
}

/// Retained view state for an [`OwnedView`].
#[derive(Debug, Clone)]
pub struct OwnedViewState<T>
where
    T: Mountable,
{
    owner: Owner,
    state: T,
}

impl<T> OwnedViewState<T>
where
    T: Mountable,
{
    /// Wraps a state with the given owner.
    fn new(state: T, owner: Owner) -> Self {
        Self { owner, state }
    }
}

impl<T> Render for OwnedView<T>
where
    T: Render,
{
    type State = OwnedViewState<T::State>;

    fn build(self) -> Self::State {
        let state = self.owner.with(|| self.view.build());
        OwnedViewState::new(state, self.owner)
    }

    fn rebuild(self, state: &mut Self::State) {
        let OwnedView { owner, view, .. } = self;
        owner.with(|| view.rebuild(&mut state.state));
        state.owner = owner;
    }
}

impl<T> AddAnyAttr for OwnedView<T>
where
    T: AddAnyAttr,
{
    type Output<SomeNewAttr: Attribute> = OwnedView<T::Output<SomeNewAttr>>;

    fn add_any_attr<NewAttr: Attribute>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        let OwnedView { owner, view } = self;
        OwnedView {
            owner,
            view: view.add_any_attr(attr),
        }
    }
}

impl<T> RenderHtml for OwnedView<T>
where
    T: RenderHtml,
{
    // TODO
    type AsyncOutput = OwnedView<T::AsyncOutput>;

    const MIN_LENGTH: usize = T::MIN_LENGTH;

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
    ) {
        self.owner.with(|| {
            self.view
                .to_html_with_buf(buf, position, escape, mark_branches)
        });
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
    ) where
        Self: Sized,
    {
        self.owner.with(|| {
            self.view.to_html_async_with_buf::<OUT_OF_ORDER>(
                buf,
                position,
                escape,
                mark_branches,
            )
        });

        // if self.owner drops here, it can be disposed before the asynchronous rendering process
        // has actually happened
        // instead, we'll stuff it into the cleanups of its parent so that it will remain alive at
        // least as long as the parent does
        Owner::on_cleanup(move || drop(self.owner));
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor,
        position: &PositionState,
    ) -> Self::State {
        let state = self
            .owner
            .with(|| self.view.hydrate::<FROM_SERVER>(cursor, position));
        OwnedViewState::new(state, self.owner)
    }

    async fn resolve(self) -> Self::AsyncOutput {
        let OwnedView { owner, view } = self;
        let view = owner
            .with(|| ScopedFuture::new(async move { view.resolve().await }))
            .await;
        OwnedView { owner, view }
    }

    fn dry_resolve(&mut self) {
        self.owner.with(|| self.view.dry_resolve());
    }
}

impl<T> Mountable for OwnedViewState<T>
where
    T: Mountable,
{
    fn unmount(&mut self) {
        self.state.unmount();
    }

    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        self.state.mount(parent, marker);
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        self.state.insert_before_this(child)
    }
}
