use crate::{
    html::attribute::{any_attribute::AnyAttribute, Attribute},
    hydration::Cursor,
    prelude::Mountable,
    ssr::StreamBuilder,
    view::{add_attr::AddAnyAttr, Position, PositionState, Render, RenderHtml},
};
use reactive_graph::{
    computed::ScopedFuture,
    owner::Owner,
};
use std::mem;

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
    // note: the drop order of the fields matters here
    // dropping `state` before `owner` ensures that cleanups happen
    // from the bottom up: i.e., the child state is dropped before
    // any other cleanups attached to this owner are fired
    state: T,
    owner: Owner,
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
        // Defer dropping the previous owner until after effects have run:
        // render effects that do things like reading from context it provides
        // may have already been triggered and queued to run. `spawn_local` will
        // defer the drop to the end of the queue, while still deterministically
        // cleaning up the memory to prevent leaks.
        let old_owner = mem::replace(&mut state.owner, owner);
        reactive_graph::spawn_local(async move { drop(old_owner) });
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
    type Owned = OwnedView<T::Owned>;

    const MIN_LENGTH: usize = T::MIN_LENGTH;

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
        extra_attrs: Vec<AnyAttribute>,
    ) {
        self.owner.with(|| {
            self.view.to_html_with_buf(
                buf,
                position,
                escape,
                mark_branches,
                extra_attrs,
            )
        });
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
        extra_attrs: Vec<AnyAttribute>,
    ) where
        Self: Sized,
    {
        self.owner.with(|| {
            self.view.to_html_async_with_buf::<OUT_OF_ORDER>(
                buf,
                position,
                escape,
                mark_branches,
                extra_attrs,
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

    async fn hydrate_async(
        self,
        cursor: &Cursor,
        position: &PositionState,
    ) -> Self::State {
        let state = self
            .owner
            .with(|| {
                ScopedFuture::new(self.view.hydrate_async(cursor, position))
            })
            .await;
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

    fn into_owned(self) -> Self::Owned {
        OwnedView {
            owner: self.owner,
            view: self.view.into_owned(),
        }
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

    fn try_mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) -> bool {
        self.state.try_mount(parent, marker)
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        self.state.insert_before_this(child)
    }

    fn elements(&self) -> Vec<crate::renderer::types::Element> {
        self.state.elements()
    }
}
