use super::{
    add_attr::AddAnyAttr, MarkBranch, Mountable, Position, PositionState,
    Render, RenderHtml,
};
use crate::{
    html::attribute::Attribute, hydration::Cursor, renderer::Renderer,
    ssr::StreamBuilder,
};
use either_of::*;
use std::marker::PhantomData;

impl<A, B, Rndr> Render<Rndr> for Either<A, B>
where
    A: Render<Rndr>,
    B: Render<Rndr>,
    Rndr: Renderer,
{
    type State = Either<A::State, B::State>;

    fn build(self) -> Self::State {
        match self {
            Either::Left(left) => Either::Left(left.build()),
            Either::Right(right) => Either::Right(right.build()),
        }
    }

    fn rebuild(self, state: &mut Self::State) {
        match (self, &mut *state) {
            (Either::Left(new), Either::Left(old)) => {
                new.rebuild(old);
            }
            (Either::Right(new), Either::Right(old)) => {
                new.rebuild(old);
            }
            (Either::Right(new), Either::Left(old)) => {
                let mut new_state = new.build();
                old.insert_before_this(&mut new_state);
                old.unmount();
                *state = Either::Right(new_state);
            }
            (Either::Left(new), Either::Right(old)) => {
                let mut new_state = new.build();
                old.insert_before_this(&mut new_state);
                old.unmount();
                *state = Either::Left(new_state);
            }
        }
    }
}

impl<A, B, Rndr> Mountable<Rndr> for Either<A, B>
where
    A: Mountable<Rndr>,
    B: Mountable<Rndr>,
    Rndr: Renderer,
{
    fn unmount(&mut self) {
        match self {
            Either::Left(left) => left.unmount(),
            Either::Right(right) => right.unmount(),
        }
    }

    fn mount(
        &mut self,
        parent: &<Rndr as Renderer>::Element,
        marker: Option<&<Rndr as Renderer>::Node>,
    ) {
        match self {
            Either::Left(left) => left.mount(parent, marker),
            Either::Right(right) => right.mount(parent, marker),
        }
    }

    fn insert_before_this(&self, child: &mut dyn Mountable<Rndr>) -> bool {
        match &self {
            Either::Left(left) => left.insert_before_this(child),
            Either::Right(right) => right.insert_before_this(child),
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

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
    ) {
        match self {
            Either::Left(left) => {
                if mark_branches {
                    buf.open_branch("0");
                }
                left.to_html_with_buf(buf, position, escape, mark_branches);
                if mark_branches {
                    buf.close_branch("0");
                }
            }
            Either::Right(right) => {
                if mark_branches {
                    buf.open_branch("1");
                }
                right.to_html_with_buf(buf, position, escape, mark_branches);
                if mark_branches {
                    buf.close_branch("1");
                }
            }
        }
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
        match self {
            Either::Left(left) => {
                if mark_branches {
                    buf.open_branch("0");
                }
                left.to_html_async_with_buf::<OUT_OF_ORDER>(
                    buf,
                    position,
                    escape,
                    mark_branches,
                );
                if mark_branches {
                    buf.close_branch("0");
                }
            }
            Either::Right(right) => {
                if mark_branches {
                    buf.open_branch("1");
                }
                right.to_html_async_with_buf::<OUT_OF_ORDER>(
                    buf,
                    position,
                    escape,
                    mark_branches,
                );
                if mark_branches {
                    buf.close_branch("1");
                }
            }
        }
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor<Rndr>,
        position: &PositionState,
    ) -> Self::State {
        match self {
            Either::Left(left) => {
                Either::Left(left.hydrate::<FROM_SERVER>(cursor, position))
            }
            Either::Right(right) => {
                Either::Right(right.hydrate::<FROM_SERVER>(cursor, position))
            }
        }
    }
}

/// Stores each value in the view state, overwriting it only if `Some(_)` is provided.
pub struct EitherKeepAlive<A, B> {
    /// The first possibility.
    pub a: Option<A>,
    /// The second possibility.
    pub b: Option<B>,
    /// If `true`, then `b` will be shown.
    pub show_b: bool,
}

/// Retained view state for [`EitherKeepAlive`].
pub struct EitherKeepAliveState<A, B> {
    a: Option<A>,
    b: Option<B>,
    showing_b: bool,
}

impl<A, B, Rndr> Render<Rndr> for EitherKeepAlive<A, B>
where
    A: Render<Rndr>,
    B: Render<Rndr>,
    Rndr: Renderer,
{
    type State = EitherKeepAliveState<A::State, B::State>;

    fn build(self) -> Self::State {
        let showing_b = self.show_b;
        let a = self.a.map(Render::build);
        let b = self.b.map(Render::build);
        EitherKeepAliveState { a, b, showing_b }
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
            (true, false) => match (&mut state.a, &mut state.b) {
                (Some(a), Some(b)) => {
                    a.insert_before_this(b);
                    a.unmount();
                }
                _ => unreachable!(),
            },
            // transition from B to A
            (false, true) => match (&mut state.a, &mut state.b) {
                (Some(a), Some(b)) => {
                    b.insert_before_this(a);
                    b.unmount();
                }
                _ => unreachable!(),
            },
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
        <A as AddAnyAttr<Rndr>>::Output<SomeNewAttr::Cloneable>,
        <B as AddAnyAttr<Rndr>>::Output<SomeNewAttr::Cloneable>,
    >;

    fn add_any_attr<NewAttr: Attribute<Rndr>>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml<Rndr>,
    {
        let EitherKeepAlive { a, b, show_b } = self;
        let attr = attr.into_cloneable();
        EitherKeepAlive {
            a: a.map(|a| a.add_any_attr(attr.clone())),
            b: b.map(|b| b.add_any_attr(attr.clone())),
            show_b,
        }
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

    fn to_html_with_buf(
        self,
        _buf: &mut String,
        _position: &mut Position,
        _escape: bool,
        _mark_branches: bool,
    ) {
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

        EitherKeepAliveState { showing_b, a, b }
    }
}

impl<A, B, Rndr> Mountable<Rndr> for EitherKeepAliveState<A, B>
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
    }

    fn insert_before_this(&self, child: &mut dyn Mountable<Rndr>) -> bool {
        if self.showing_b {
            self.b
                .as_ref()
                .expect("B was not present")
                .insert_before_this(child)
        } else {
            self.a
                .as_ref()
                .expect("A was not present")
                .insert_before_this(child)
        }
    }
}

macro_rules! tuples {
    ($num:literal => $($ty:ident),*) => {
        paste::paste! {
            #[doc = concat!("Retained view state for ", stringify!([<EitherOf $num>]), ".")]
            pub struct [<EitherOf $num State>]<$($ty,)* Rndr>
            where
                $($ty: Render<Rndr>,)*
                Rndr: Renderer
            {
                /// Which child view state is being displayed.
                pub state: [<EitherOf $num>]<$($ty::State,)*>,
                /// The renderer.
                pub rndr: PhantomData<Rndr>
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
                }

                fn mount(
                    &mut self,
                    parent: &<Rndr as Renderer>::Element,
                    marker: Option<&<Rndr as Renderer>::Node>,
                ) {
                    match &mut self.state {
                        $([<EitherOf $num>]::$ty(this) => [<EitherOf $num>]::$ty(this.mount(parent, marker)),)*
                    };
                }

                fn insert_before_this(&self,
                    child: &mut dyn Mountable<Rndr>,
                ) -> bool {
                    match &self.state {
                        $([<EitherOf $num>]::$ty(this) =>this.insert_before_this(child),)*
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
                    let state = match self {
                        $([<EitherOf $num>]::$ty(this) => [<EitherOf $num>]::$ty(this.build()),)*
                    };
                    Self::State { state, rndr: PhantomData }
                }

                fn rebuild(self, state: &mut Self::State) {
                    let new_state = match (self, &mut state.state) {
                        // rebuild same state and return early
                        $(([<EitherOf $num>]::$ty(new), [<EitherOf $num>]::$ty(old)) => { return new.rebuild(old); },)*
                        // or mount new state
                        $(([<EitherOf $num>]::$ty(new), _) => {
                            let mut new = new.build();
                            state.insert_before_this(&mut new);
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

                fn to_html_with_buf(self, buf: &mut String, position: &mut Position, escape: bool, mark_branches: bool) {
                    match self {
                        $([<EitherOf $num>]::$ty(this) => {
                            if mark_branches {
                                buf.open_branch(stringify!($ty));
                            }
                            this.to_html_with_buf(buf, position, escape, mark_branches);
                            if mark_branches {
                                buf.close_branch(stringify!($ty));
                            }
                        })*
                    }
                }

                fn to_html_async_with_buf<const OUT_OF_ORDER: bool>(
                    self,
                    buf: &mut StreamBuilder, position: &mut Position, escape: bool, mark_branches: bool) where
                    Self: Sized,
                {
                    match self {
                        $([<EitherOf $num>]::$ty(this) => {
                            if mark_branches {
                                buf.open_branch(stringify!($ty));
                            }
                            this.to_html_async_with_buf::<OUT_OF_ORDER>(buf, position, escape, mark_branches);
                            if mark_branches {
                                buf.close_branch(stringify!($ty));
                            }
                        })*
                    }
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

                    Self::State { state, rndr: PhantomData }
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
