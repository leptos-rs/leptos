use super::{
    add_attr::AddAnyAttr, Mountable, Position, PositionState, Render,
    RenderHtml,
};
use crate::{
    html::attribute::Attribute, hydration::Cursor, renderer::Renderer,
    ssr::StreamBuilder,
};
use either_of::*;

pub struct EitherState<A, B, Rndr>
where
    A: Mountable<Rndr>,
    B: Mountable<Rndr>,
    Rndr: Renderer,
{
    pub state: Either<A, B>,
    pub marker: Rndr::Placeholder,
}

impl<A, B, Rndr> Render<Rndr> for Either<A, B>
where
    A: Render<Rndr>,
    B: Render<Rndr>,
    Rndr: Renderer,
{
    type State = EitherState<A::State, B::State, Rndr>;

    fn build(self) -> Self::State {
        let marker = Rndr::create_placeholder();
        match self {
            Either::Left(left) => EitherState {
                state: Either::Left(left.build()),
                marker,
            },
            Either::Right(right) => EitherState {
                state: Either::Right(right.build()),
                marker,
            },
        }
    }

    fn rebuild(self, state: &mut Self::State) {
        let marker = state.marker.as_ref();
        match (self, &mut state.state) {
            (Either::Left(new), Either::Left(old)) => new.rebuild(old),
            (Either::Right(new), Either::Right(old)) => new.rebuild(old),
            (Either::Right(new), Either::Left(old)) => {
                old.unmount();
                let mut new_state = new.build();
                Rndr::try_mount_before(&mut new_state, marker);
                state.state = Either::Right(new_state);
            }
            (Either::Left(new), Either::Right(old)) => {
                old.unmount();
                let mut new_state = new.build();
                Rndr::try_mount_before(&mut new_state, marker);
                state.state = Either::Left(new_state);
            }
        }
    }
}

impl<A, B, Rndr> Mountable<Rndr> for EitherState<A, B, Rndr>
where
    A: Mountable<Rndr>,
    B: Mountable<Rndr>,
    Rndr: Renderer,
{
    fn unmount(&mut self) {
        match &mut self.state {
            Either::Left(left) => left.unmount(),
            Either::Right(right) => right.unmount(),
        }
        self.marker.unmount();
    }

    fn mount(
        &mut self,
        parent: &<Rndr as Renderer>::Element,
        marker: Option<&<Rndr as Renderer>::Node>,
    ) {
        self.marker.mount(parent, marker);
        match &mut self.state {
            Either::Left(left) => {
                left.mount(parent, Some(self.marker.as_ref()))
            }
            Either::Right(right) => {
                right.mount(parent, Some(self.marker.as_ref()))
            }
        }
    }

    fn insert_before_this(
        &self,
        parent: &<Rndr as Renderer>::Element,
        child: &mut dyn Mountable<Rndr>,
    ) -> bool {
        match &self.state {
            Either::Left(left) => left.insert_before_this(parent, child),
            Either::Right(right) => right.insert_before_this(parent, child),
        }
    }
}

impl<A, B, Rndr> AddAnyAttr<Rndr> for Either<A, B>
where
    A: RenderHtml<Rndr>,
    B: RenderHtml<Rndr>,
    Rndr: Renderer,
{
    type Output<SomeNewAttr: Attribute<Rndr>> = Either<
        <A as AddAnyAttr<Rndr>>::Output<SomeNewAttr>,
        <B as AddAnyAttr<Rndr>>::Output<SomeNewAttr>,
    >;

    fn add_any_attr<NewAttr: Attribute<Rndr>>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<Rndr>,
    {
        match self {
            Either::Left(i) => Either::Left(i.add_any_attr(attr)),
            Either::Right(i) => Either::Right(i.add_any_attr(attr)),
        }
    }
}

const fn max_usize(vals: &[usize]) -> usize {
    let mut max = 0;
    let len = vals.len();
    let mut i = 0;
    while i < len {
        if vals[i] > max {
            max = vals[i];
        }
        i += 1;
    }
    max
}

impl<A, B, Rndr> RenderHtml<Rndr> for Either<A, B>
where
    A: RenderHtml<Rndr>,
    B: RenderHtml<Rndr>,
    Rndr: Renderer,
{
    type AsyncOutput = Either<A::AsyncOutput, B::AsyncOutput>;

    fn dry_resolve(&mut self) {
        match self {
            Either::Left(left) => left.dry_resolve(),
            Either::Right(right) => right.dry_resolve(),
        }
    }

    async fn resolve(self) -> Self::AsyncOutput {
        match self {
            Either::Left(left) => Either::Left(left.resolve().await),
            Either::Right(right) => Either::Right(right.resolve().await),
        }
    }

    const MIN_LENGTH: usize = max_usize(&[A::MIN_LENGTH, B::MIN_LENGTH]);

    #[inline(always)]
    fn html_len(&self) -> usize {
        match self {
            Either::Left(i) => i.html_len(),
            Either::Right(i) => i.html_len(),
        }
    }

    fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
        match self {
            Either::Left(left) => left.to_html_with_buf(buf, position),
            Either::Right(right) => right.to_html_with_buf(buf, position),
        }
        buf.push_str("<!>");
        *position = Position::NextChild;
    }

    fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
        self,
        buf: &mut StreamBuilder,
        position: &mut Position,
    ) where
        Self: Sized,
    {
        match self {
            Either::Left(left) => {
                left.to_html_async_with_buf::<OUT_OF_ORDER>(buf, position)
            }
            Either::Right(right) => {
                right.to_html_async_with_buf::<OUT_OF_ORDER>(buf, position)
            }
        }
        buf.push_sync("<!>");
        *position = Position::NextChild;
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<Rndr>,
        position: &PositionState,
    ) -> Self::State {
        let state = match self {
            Either::Left(left) => {
                Either::Left(left.hydrate::<FROM_SERVER>(cursor, position))
            }
            Either::Right(right) => {
                Either::Right(right.hydrate::<FROM_SERVER>(cursor, position))
            }
        };

        let marker = cursor.next_placeholder(position);

        EitherState { state, marker }
    }
}

/// Stores each value in the view state, overwriting it only if `Some(_)` is provided.
pub struct EitherKeepAlive<A, B> {
    pub a: Option<A>,
    pub b: Option<B>,
    pub show_b: bool,
}

pub struct EitherKeepAliveState<A, B, Rndr>
where
    Rndr: Renderer,
{
    a: Option<A>,
    b: Option<B>,
    showing_b: bool,
    marker: Rndr::Placeholder,
}

impl<A, B, Rndr> Render<Rndr> for EitherKeepAlive<A, B>
where
    A: Render<Rndr>,
    B: Render<Rndr>,
    Rndr: Renderer,
{
    type State = EitherKeepAliveState<A::State, B::State, Rndr>;

    fn build(self) -> Self::State {
        let marker = Rndr::create_placeholder();
        let showing_b = self.show_b;
        let a = self.a.map(Render::build);
        let b = self.b.map(Render::build);
        EitherKeepAliveState {
            a,
            b,
            showing_b,
            marker,
        }
    }

    fn rebuild(self, state: &mut Self::State) {
        // set or update A -- `None` just means "no change"
        match (self.a, &mut state.a) {
            (Some(new), Some(old)) => new.rebuild(old),
            (Some(new), None) => state.a = Some(new.build()),
            _ => {}
        }

        // set or update B
        match (self.b, &mut state.b) {
            (Some(new), Some(old)) => new.rebuild(old),
            (Some(new), None) => state.b = Some(new.build()),
            _ => {}
        }

        match (self.show_b, state.showing_b) {
            // transition from A to B
            (true, false) => {
                if let Some(a) = &mut state.a {
                    a.unmount();
                }
                if let Some(b) = &mut state.b {
                    Rndr::mount_before(b, state.marker.as_ref());
                }
            }
            // transition from B to A
            (false, true) => {
                if let Some(b) = &mut state.b {
                    b.unmount();
                }
                if let Some(a) = &mut state.a {
                    Rndr::mount_before(a, state.marker.as_ref());
                }
            }
            _ => {}
        }
        state.showing_b = self.show_b;
    }
}

impl<A, B, Rndr> AddAnyAttr<Rndr> for EitherKeepAlive<A, B>
where
    A: RenderHtml<Rndr>,
    B: RenderHtml<Rndr>,
    Rndr: Renderer,
{
    type Output<SomeNewAttr: Attribute<Rndr>> = EitherKeepAlive<
        <A as AddAnyAttr<Rndr>>::Output<SomeNewAttr>,
        <B as AddAnyAttr<Rndr>>::Output<SomeNewAttr>,
    >;

    fn add_any_attr<NewAttr: Attribute<Rndr>>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<Rndr>,
    {
        todo!()
    }
}

impl<A, B, Rndr> RenderHtml<Rndr> for EitherKeepAlive<A, B>
where
    A: RenderHtml<Rndr>,
    B: RenderHtml<Rndr>,
    Rndr: Renderer,
{
    type AsyncOutput = Either<A::AsyncOutput, B::AsyncOutput>;

    const MIN_LENGTH: usize = 0;

    fn dry_resolve(&mut self) {
        todo!()
    }

    async fn resolve(self) -> Self::AsyncOutput {
        todo!()
    }

    fn to_html_with_buf(self, _buf: &mut String, _position: &mut Position) {
        todo!()
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<Rndr>,
        position: &PositionState,
    ) -> Self::State {
        let showing_b = self.show_b;
        let a = self.a.map(|a| {
            if showing_b {
                a.build()
            } else {
                a.hydrate::<FROM_SERVER>(cursor, position)
            }
        });
        let b = self.b.map(|b| {
            if showing_b {
                b.hydrate::<FROM_SERVER>(cursor, position)
            } else {
                b.build()
            }
        });

        let marker = cursor.next_placeholder(position);

        EitherKeepAliveState {
            showing_b,
            a,
            b,
            marker,
        }
    }
}

impl<A, B, Rndr> Mountable<Rndr> for EitherKeepAliveState<A, B, Rndr>
where
    A: Mountable<Rndr>,
    B: Mountable<Rndr>,
    Rndr: Renderer,
{
    fn unmount(&mut self) {
        if self.showing_b {
            self.b.as_mut().expect("B was not present").unmount();
        } else {
            self.a.as_mut().expect("A was not present").unmount();
        }
        self.marker.unmount();
    }

    fn mount(
        &mut self,
        parent: &<Rndr as Renderer>::Element,
        marker: Option<&<Rndr as Renderer>::Node>,
    ) {
        if self.showing_b {
            self.b
                .as_mut()
                .expect("B was not present")
                .mount(parent, marker);
        } else {
            self.a
                .as_mut()
                .expect("A was not present")
                .mount(parent, marker);
        }
        self.marker.mount(parent, marker);
    }

    fn insert_before_this(
        &self,
        parent: &<Rndr as Renderer>::Element,
        child: &mut dyn Mountable<Rndr>,
    ) -> bool {
        if self.showing_b {
            self.b
                .as_ref()
                .expect("B was not present")
                .insert_before_this(parent, child)
        } else {
            self.a
                .as_ref()
                .expect("A was no present")
                .insert_before_this(parent, child)
        }
    }
}

macro_rules! tuples {
    ($num:literal => $($ty:ident),*) => {
        paste::paste! {
            pub struct [<EitherOf $num State>]<$($ty,)* Rndr>
            where
                $($ty: Render<Rndr>,)*
                Rndr: Renderer
            {
                pub state: [<EitherOf $num>]<$($ty::State,)*>,
                pub marker: Rndr::Placeholder,
            }

            impl<$($ty,)* Rndr> Mountable<Rndr> for [<EitherOf $num State>]<$($ty,)* Rndr>
            where
                $($ty: Render<Rndr>,)*
                Rndr: Renderer
            {
                fn unmount(&mut self) {
                    match &mut self.state {
                        $([<EitherOf $num>]::$ty(this) => [<EitherOf $num>]::$ty(this.unmount()),)*
                    };
                    self.marker.unmount();
                }

                fn mount(
                    &mut self,
                    parent: &<Rndr as Renderer>::Element,
                    marker: Option<&<Rndr as Renderer>::Node>,
                ) {
                    self.marker.mount(parent, marker);
                    match &mut self.state {
                        $([<EitherOf $num>]::$ty(this) => [<EitherOf $num>]::$ty(this.mount(parent, Some(self.marker.as_ref()))),)*
                    };
                }

                fn insert_before_this(
                    &self,
                    parent: &<Rndr as Renderer>::Element,
                    child: &mut dyn Mountable<Rndr>,
                ) -> bool {
                    match &self.state {
                        $([<EitherOf $num>]::$ty(this) =>this.insert_before_this(parent, child),)*
                    }
                }
            }

            impl<Rndr, $($ty,)*> Render<Rndr> for [<EitherOf $num>]<$($ty,)*>
            where
                $($ty: Render<Rndr>,)*
                Rndr: Renderer
            {
                type State = [<EitherOf $num State>]<$($ty,)* Rndr>;


                fn build(self) -> Self::State {
                    let marker = Rndr::create_placeholder();
                    let state = match self {
                        $([<EitherOf $num>]::$ty(this) => [<EitherOf $num>]::$ty(this.build()),)*
                    };
                    Self::State { marker, state }
                }

                fn rebuild(self, state: &mut Self::State) {
                    let marker = state.marker.as_ref();
                    let new_state = match (self, &mut state.state) {
                        // rebuild same state and return early
                        $(([<EitherOf $num>]::$ty(new), [<EitherOf $num>]::$ty(old)) => { return new.rebuild(old); },)*
                        // or mount new state
                        $(([<EitherOf $num>]::$ty(new), _) => {
                            let mut new = new.build();
                            Rndr::try_mount_before(&mut new, marker);
                            [<EitherOf $num>]::$ty(new)
                        },)*
                    };

                    // and then unmount old state
                    match &mut state.state {
                        $([<EitherOf $num>]::$ty(this) => this.unmount(),)*
                    };

                    // and store the new state
                    state.state = new_state;
                }
            }

            impl<Rndr, $($ty,)*> AddAnyAttr<Rndr> for [<EitherOf $num>]<$($ty,)*>
            where
                $($ty: RenderHtml<Rndr>,)*
                Rndr: Renderer,
            {
                type Output<SomeNewAttr: Attribute<Rndr>> = [<EitherOf $num>]<
                    $(<$ty as AddAnyAttr<Rndr>>::Output<SomeNewAttr>,)*
                >;

                fn add_any_attr<NewAttr: Attribute<Rndr>>(
                    self,
                    attr: NewAttr,
                ) -> Self::Output<NewAttr>
                where
                    Self::Output<NewAttr>: RenderHtml<Rndr>,
                {
                    match self {
                        $([<EitherOf $num>]::$ty(this) => [<EitherOf $num>]::$ty(this.add_any_attr(attr)),)*
                    }
                }
            }

            impl<Rndr, $($ty,)*> RenderHtml<Rndr> for [<EitherOf $num>]<$($ty,)*>
            where
                $($ty: RenderHtml<Rndr>,)*
                Rndr: Renderer,
            {
                type AsyncOutput = [<EitherOf $num>]<$($ty::AsyncOutput,)*>;

                const MIN_LENGTH: usize = max_usize(&[$($ty ::MIN_LENGTH,)*]);


                fn dry_resolve(&mut self) {
                    match self {
                        $([<EitherOf $num>]::$ty(this) => {
                            this.dry_resolve();
                        })*
                    }
                }

                async fn resolve(self) -> Self::AsyncOutput {
                    match self {
                        $([<EitherOf $num>]::$ty(this) => [<EitherOf $num>]::$ty(this.resolve().await),)*
                    }
                }

                #[inline(always)]
                fn html_len(&self) -> usize {
                    match self {
                        $([<EitherOf $num>]::$ty(i) => i.html_len(),)*
                    }
                }

                fn to_html_with_buf(self, buf: &mut String, position: &mut Position) {
                    match self {
                        $([<EitherOf $num>]::$ty(this) => this.to_html_with_buf(buf, position),)*
                    }
                    buf.push_str("<!>");
                    *position = Position::NextChild;
                }

                fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
                    self,
                    buf: &mut StreamBuilder,
                    position: &mut Position,
                ) where
                    Self: Sized,
                {
                    match self {
                        $([<EitherOf $num>]::$ty(this) => this.to_html_async_with_buf::<OUT_OF_ORDER>(buf, position),)*
                    }
                    buf.push_sync("<!>");
                    *position = Position::NextChild;
                }

                fn hydrate<const FROM_SERVER: bool>(
                    self,
                    cursor: &Cursor<Rndr>,
                    position: &PositionState,
                ) -> Self::State {
                    let state = match self {
                        $([<EitherOf $num>]::$ty(this) => {
                            [<EitherOf $num>]::$ty(this.hydrate::<FROM_SERVER>(cursor, position))
                        })*
                    };

                    let marker = cursor.next_placeholder(position);
                    Self::State { marker, state }
                }
            }
        }
    }
}

tuples!(3 => A, B, C);
tuples!(4 => A, B, C, D);
tuples!(5 => A, B, C, D, E);
tuples!(6 => A, B, C, D, E, F);
tuples!(7 => A, B, C, D, E, F, G);
tuples!(8 => A, B, C, D, E, F, G, H);
tuples!(9 => A, B, C, D, E, F, G, H, I);
tuples!(10 => A, B, C, D, E, F, G, H, I, J);
tuples!(11 => A, B, C, D, E, F, G, H, I, J, K);
tuples!(12 => A, B, C, D, E, F, G, H, I, J, K, L);
tuples!(13 => A, B, C, D, E, F, G, H, I, J, K, L, M);
tuples!(14 => A, B, C, D, E, F, G, H, I, J, K, L, M, N);
tuples!(15 => A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
tuples!(16 => A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
