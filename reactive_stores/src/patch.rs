use crate::{path::StorePath, StoreField};
use itertools::{EitherOrBoth, Itertools};
use reactive_graph::traits::{Notify, UntrackableGuard};
use std::{
    borrow::Cow,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    num::{
        NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8,
        NonZeroIsize, NonZeroU128, NonZeroU16, NonZeroU32, NonZeroU64,
        NonZeroU8, NonZeroUsize,
    },
    rc::Rc,
    sync::Arc,
};

/// Allows updating a store or field in place with a new value.
pub trait Patch {
    /// The type of the new value.
    type Value;

    /// Patches a store or field with a new value, only notifying fields that have changed.
    fn patch(&self, new: Self::Value);
}

impl<T> Patch for T
where
    T: StoreField,
    T::Value: PatchField,
{
    type Value = T::Value;

    fn patch(&self, new: Self::Value) {
        let path = self.path().into_iter().collect::<StorePath>();
        if let Some(mut writer) = self.writer() {
            // don't track the writer for the whole store
            writer.untrack();
            let mut notify = |path: &StorePath| {
                self.get_trigger(path.to_owned()).this.notify();
                self.get_trigger(path.to_owned()).children.notify();
            };
            writer.patch_field(new, &path, &mut notify);
        }
    }
}

/// Allows patching a store field with some new value.
pub trait PatchField {
    /// Patches the field with some new value, only notifying if the value has changed.
    fn patch_field(
        &mut self,
        new: Self,
        path: &StorePath,
        notify: &mut dyn FnMut(&StorePath),
    );
}

macro_rules! patch_primitives {
    ($($ty:ty),*) => {
        $(impl PatchField for $ty {
            fn patch_field(
                &mut self,
                new: Self,
                path: &StorePath,
                notify: &mut dyn FnMut(&StorePath),
            ) {
                if new != *self {
                    *self = new;
                    notify(path);
                }
            }
        })*
    };
}

patch_primitives! {
    &str,
    String,
    Arc<str>,
    Rc<str>,
    Cow<'_, str>,
    usize,
    u8,
    u16,
    u32,
    u64,
    u128,
    isize,
    i8,
    i16,
    i32,
    i64,
    i128,
    f32,
    f64,
    char,
    bool,
    IpAddr,
    SocketAddr,
    SocketAddrV4,
    SocketAddrV6,
    Ipv4Addr,
    Ipv6Addr,
    NonZeroI8,
    NonZeroU8,
    NonZeroI16,
    NonZeroU16,
    NonZeroI32,
    NonZeroU32,
    NonZeroI64,
    NonZeroU64,
    NonZeroI128,
    NonZeroU128,
    NonZeroIsize,
    NonZeroUsize
}

impl<T> PatchField for Option<T>
where
    T: PatchField,
{
    fn patch_field(
        &mut self,
        new: Self,
        path: &StorePath,
        notify: &mut dyn FnMut(&StorePath),
    ) {
        match (self, new) {
            (None, None) => {}
            (old @ Some(_), None) => {
                old.take();
                notify(path);
            }
            (old @ None, new @ Some(_)) => {
                *old = new;
                notify(path);
            }
            (Some(old), Some(new)) => {
                let mut new_path = path.to_owned();
                new_path.push(0);
                old.patch_field(new, &new_path, notify);
            }
        }
    }
}

impl<T> PatchField for Vec<T>
where
    T: PatchField,
{
    fn patch_field(
        &mut self,
        new: Self,
        path: &StorePath,
        notify: &mut dyn FnMut(&StorePath),
    ) {
        if self.is_empty() && new.is_empty() {
            return;
        }

        if new.is_empty() {
            self.clear();
            notify(path);
        } else if self.is_empty() {
            self.extend(new);
            notify(path);
        } else {
            let mut adds = vec![];
            let mut removes_at_end = 0;
            let mut new_path = path.to_owned();
            new_path.push(0);
            for (idx, item) in
                new.into_iter().zip_longest(self.iter_mut()).enumerate()
            {
                match item {
                    EitherOrBoth::Both(new, old) => {
                        old.patch_field(new, &new_path, notify);
                    }
                    EitherOrBoth::Left(new) => {
                        adds.push(new);
                    }
                    EitherOrBoth::Right(_) => {
                        removes_at_end += 1;
                    }
                }
                new_path.replace_last(idx + 1);
            }

            let length_changed = removes_at_end > 0 || !adds.is_empty();
            self.truncate(self.len() - removes_at_end);
            self.append(&mut adds);

            if length_changed {
                notify(path);
            }
        }
    }
}

macro_rules! patch_tuple {
	($($ty:ident),*) => {
		impl<$($ty),*> PatchField for ($($ty,)*)
		where
			$($ty: PatchField),*,
		{
            fn patch_field(
                &mut self,
                new: Self,
                path: &StorePath,
                notify: &mut dyn FnMut(&StorePath),
            ) {
                let mut idx = 0;
                let mut new_path = path.to_owned();
                new_path.push(0);

                paste::paste! {
                    #[allow(non_snake_case)]
                    let ($($ty,)*) = self;
                    let ($([<new_ $ty:lower>],)*) = new;
                    $(
                        $ty.patch_field([<new_ $ty:lower>], &new_path, notify);
                        idx += 1;
                        new_path.replace_last(idx);
                    )*
                }
            }
        }
    }
}

impl PatchField for () {
    fn patch_field(
        &mut self,
        _new: Self,
        _path: &StorePath,
        _notify: &mut dyn FnMut(&StorePath),
    ) {
    }
}

patch_tuple!(A);
patch_tuple!(A, B);
patch_tuple!(A, B, C);
patch_tuple!(A, B, C, D);
patch_tuple!(A, B, C, D, E);
patch_tuple!(A, B, C, D, E, F);
patch_tuple!(A, B, C, D, E, F, G);
patch_tuple!(A, B, C, D, E, F, G, H);
patch_tuple!(A, B, C, D, E, F, G, H, I);
patch_tuple!(A, B, C, D, E, F, G, H, I, J);
patch_tuple!(A, B, C, D, E, F, G, H, I, J, K);
patch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
patch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
patch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
patch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
patch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
patch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q);
patch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R);
patch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S);
patch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T);
patch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U);
patch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V);
patch_tuple!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W
);
patch_tuple!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X
);
patch_tuple!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y
);
patch_tuple!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y,
    Z
);
