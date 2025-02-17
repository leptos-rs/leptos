use super::{
    add_attr::AddAnyAttr, MarkBranch, Mountable, Position, PositionState,
    Render, RenderHtml,
};
use crate::{
    html::attribute::{Attribute, NextAttribute},
    hydration::Cursor,
    ssr::StreamBuilder,
};
use either_of::*;
use futures::future::join;

impl<A, B> Render for Either<A, B>
where
    A: Render,
    B: Render,
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

impl<A, B> Mountable for Either<A, B>
where
    A: Mountable,
    B: Mountable,
{
    fn unmount(&mut self) {
        match self {
            Either::Left(left) => left.unmount(),
            Either::Right(right) => right.unmount(),
        }
    }

    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        match self {
            Either::Left(left) => left.mount(parent, marker),
            Either::Right(right) => right.mount(parent, marker),
        }
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        match &self {
            Either::Left(left) => left.insert_before_this(child),
            Either::Right(right) => right.insert_before_this(child),
        }
    }
}

impl<A, B> AddAnyAttr for Either<A, B>
where
    A: RenderHtml,
    B: RenderHtml,
{
    type Output<SomeNewAttr: Attribute> = Either<
        <A as AddAnyAttr>::Output<SomeNewAttr>,
        <B as AddAnyAttr>::Output<SomeNewAttr>,
    >;

    fn add_any_attr<NewAttr: Attribute>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
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

#[cfg(not(erase_components))]
impl<A, B> NextAttribute for Either<A, B>
where
    B: NextAttribute,
    A: NextAttribute,
{
    type Output<NewAttr: Attribute> = Either<
        <A as NextAttribute>::Output<NewAttr>,
        <B as NextAttribute>::Output<NewAttr>,
    >;

    fn add_any_attr<NewAttr: Attribute>(
        self,
        new_attr: NewAttr,
    ) -> Self::Output<NewAttr> {
        match self {
            Either::Left(left) => Either::Left(left.add_any_attr(new_attr)),
            Either::Right(right) => Either::Right(right.add_any_attr(new_attr)),
        }
    }
}

#[cfg(erase_components)]
use crate::html::attribute::any_attribute::{AnyAttribute, IntoAnyAttribute};

#[cfg(erase_components)]
impl<A, B> NextAttribute for Either<A, B>
where
    B: IntoAnyAttribute,
    A: IntoAnyAttribute,
{
    type Output<NewAttr: Attribute> = Vec<AnyAttribute>;

    fn add_any_attr<NewAttr: Attribute>(
        self,
        new_attr: NewAttr,
    ) -> Self::Output<NewAttr> {
        vec![
            match self {
                Either::Left(left) => left.into_any_attr(),
                Either::Right(right) => right.into_any_attr(),
            },
            new_attr.into_any_attr(),
        ]
    }
}

impl<A, B> Attribute for Either<A, B>
where
    B: Attribute,
    A: Attribute,
{
    const MIN_LENGTH: usize = max_usize(&[A::MIN_LENGTH, B::MIN_LENGTH]);

    type AsyncOutput = Either<A::AsyncOutput, B::AsyncOutput>;
    type State = Either<A::State, B::State>;
    type Cloneable = Either<A::Cloneable, B::Cloneable>;
    type CloneableOwned = Either<A::CloneableOwned, B::CloneableOwned>;

    fn html_len(&self) -> usize {
        match self {
            Either::Left(left) => left.html_len(),
            Either::Right(right) => right.html_len(),
        }
    }

    fn to_html(
        self,
        buf: &mut String,
        class: &mut String,
        style: &mut String,
        inner_html: &mut String,
    ) {
        match self {
            Either::Left(left) => left.to_html(buf, class, style, inner_html),
            Either::Right(right) => {
                right.to_html(buf, class, style, inner_html)
            }
        }
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        el: &crate::renderer::types::Element,
    ) -> Self::State {
        match self {
            Either::Left(left) => Either::Left(left.hydrate::<FROM_SERVER>(el)),
            Either::Right(right) => {
                Either::Right(right.hydrate::<FROM_SERVER>(el))
            }
        }
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        match self {
            Either::Left(left) => Either::Left(left.build(el)),
            Either::Right(right) => Either::Right(right.build(el)),
        }
    }

    fn rebuild(self, state: &mut Self::State) {
        match self {
            Either::Left(left) => {
                if let Some(state) = state.as_left_mut() {
                    left.rebuild(state)
                }
            }
            Either::Right(right) => {
                if let Some(state) = state.as_right_mut() {
                    right.rebuild(state)
                }
            }
        }
    }

    fn into_cloneable(self) -> Self::Cloneable {
        match self {
            Either::Left(left) => Either::Left(left.into_cloneable()),
            Either::Right(right) => Either::Right(right.into_cloneable()),
        }
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        match self {
            Either::Left(left) => Either::Left(left.into_cloneable_owned()),
            Either::Right(right) => Either::Right(right.into_cloneable_owned()),
        }
    }

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
}

impl<A, B> RenderHtml for Either<A, B>
where
    A: RenderHtml,
    B: RenderHtml,
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
        cursor: &Cursor,
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

impl<A, B> Render for EitherKeepAlive<A, B>
where
    A: Render,
    B: Render,
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

impl<A, B> AddAnyAttr for EitherKeepAlive<A, B>
where
    A: RenderHtml,
    B: RenderHtml,
{
    type Output<SomeNewAttr: Attribute> = EitherKeepAlive<
        <A as AddAnyAttr>::Output<SomeNewAttr::Cloneable>,
        <B as AddAnyAttr>::Output<SomeNewAttr::Cloneable>,
    >;

    fn add_any_attr<NewAttr: Attribute>(
        self,
        attr: NewAttr,
    ) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
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

impl<A, B> RenderHtml for EitherKeepAlive<A, B>
where
    A: RenderHtml,
    B: RenderHtml,
{
    type AsyncOutput = EitherKeepAlive<A::AsyncOutput, B::AsyncOutput>;

    const MIN_LENGTH: usize = 0;

    fn dry_resolve(&mut self) {
        if let Some(inner) = &mut self.a {
            inner.dry_resolve();
        }
        if let Some(inner) = &mut self.b {
            inner.dry_resolve();
        }
    }

    async fn resolve(self) -> Self::AsyncOutput {
        let EitherKeepAlive { a, b, show_b } = self;
        let (a, b) = join(
            async move {
                match a {
                    Some(a) => Some(a.resolve().await),
                    None => None,
                }
            },
            async move {
                match b {
                    Some(b) => Some(b.resolve().await),
                    None => None,
                }
            },
        )
        .await;
        EitherKeepAlive { a, b, show_b }
    }

    fn to_html_with_buf(
        self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
    ) {
        if self.show_b {
            self.b
                .expect("rendering B to HTML without filling it")
                .to_html_with_buf(buf, position, escape, mark_branches);
        } else {
            self.a
                .expect("rendering A to HTML without filling it")
                .to_html_with_buf(buf, position, escape, mark_branches);
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
        if self.show_b {
            self.b
                .expect("rendering B to HTML without filling it")
                .to_html_async_with_buf::<OUT_OF_ORDER>(
                    buf,
                    position,
                    escape,
                    mark_branches,
                );
        } else {
            self.a
                .expect("rendering A to HTML without filling it")
                .to_html_async_with_buf::<OUT_OF_ORDER>(
                    buf,
                    position,
                    escape,
                    mark_branches,
                );
        }
    }

    fn hydrate<const FROM_SERVER: bool>(
        self,
        cursor: &Cursor,
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

impl<A, B> Mountable for EitherKeepAliveState<A, B>
where
    A: Mountable,
    B: Mountable,
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
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
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

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
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
            pub struct [<EitherOf $num State>]<$($ty,)*>
            where
                $($ty: Render,)*

            {
                /// Which child view state is being displayed.
                pub state: [<EitherOf $num>]<$($ty::State,)*>,
            }

            impl<$($ty,)*> Mountable for [<EitherOf $num State>]<$($ty,)*>
            where
                $($ty: Render,)*

            {
                fn unmount(&mut self) {
                    match &mut self.state {
                        $([<EitherOf $num>]::$ty(this) => [<EitherOf $num>]::$ty(this.unmount()),)*
                    };
                }

                fn mount(
                    &mut self,
                    parent: &crate::renderer::types::Element,
                    marker: Option<&crate::renderer::types::Node>,
                ) {
                    match &mut self.state {
                        $([<EitherOf $num>]::$ty(this) => [<EitherOf $num>]::$ty(this.mount(parent, marker)),)*
                    };
                }

                fn insert_before_this(&self,
                    child: &mut dyn Mountable,
                ) -> bool {
                    match &self.state {
                        $([<EitherOf $num>]::$ty(this) =>this.insert_before_this(child),)*
                    }
                }
            }

            impl<$($ty,)*> Render for [<EitherOf $num>]<$($ty,)*>
            where
                $($ty: Render,)*

            {
                type State = [<EitherOf $num State>]<$($ty,)*>;


                fn build(self) -> Self::State {
                    let state = match self {
                        $([<EitherOf $num>]::$ty(this) => [<EitherOf $num>]::$ty(this.build()),)*
                    };
                    Self::State { state }
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

            impl<$($ty,)*> AddAnyAttr for [<EitherOf $num>]<$($ty,)*>
            where
                $($ty: RenderHtml,)*

            {
                type Output<SomeNewAttr: Attribute> = [<EitherOf $num>]<
                    $(<$ty as AddAnyAttr>::Output<SomeNewAttr>,)*
                >;

                fn add_any_attr<NewAttr: Attribute>(
                    self,
                    attr: NewAttr,
                ) -> Self::Output<NewAttr>
                where
                    Self::Output<NewAttr>: RenderHtml,
                {
                    match self {
                        $([<EitherOf $num>]::$ty(this) => [<EitherOf $num>]::$ty(this.add_any_attr(attr)),)*
                    }
                }
            }

            impl<$($ty,)*> RenderHtml for [<EitherOf $num>]<$($ty,)*>
            where
                $($ty: RenderHtml,)*

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
                    cursor: &Cursor,
                    position: &PositionState,
                ) -> Self::State {
                    let state = match self {
                        $([<EitherOf $num>]::$ty(this) => {
                            [<EitherOf $num>]::$ty(this.hydrate::<FROM_SERVER>(cursor, position))
                        })*
                    };

                    Self::State { state }
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
