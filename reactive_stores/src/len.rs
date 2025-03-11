use std::{
    borrow::Cow,
    collections::{LinkedList, VecDeque},
};

/// A trait for getting the length of a collection.
pub trait Len {
    /// Returns the length of the collection.
    fn len(&self) -> usize;
}

macro_rules! delegate_impl_len {
    (<$($lt: lifetime,)*$($generics: ident,)*> $ty:ty) => {
        impl<$($lt,)*$($generics,)*> Len for $ty {
            #[inline(always)]
            fn len(&self) -> usize {
                <$ty>::len(self)
            }
        }

        impl<$($lt,)*$($generics,)*> Len for &$ty {
            #[inline(always)]
            fn len(&self) -> usize {
                Len::len(*self)
            }
        }

        impl<$($lt,)*$($generics,)*> Len for &mut $ty {
            #[inline(always)]
            fn len(&self) -> usize {
                Len::len(*self)
            }
        }
    };
    ($ty:ty) => {
        delegate_impl_len!(<> $ty);
    };
}

delegate_impl_len!(<T,> [T]);
delegate_impl_len!(<T,> Vec<T>);
delegate_impl_len!(str);
delegate_impl_len!(String);

impl<'a> Len for Cow<'a, str> {
    #[inline(always)]
    fn len(&self) -> usize {
        <str>::len(self)
    }
}

impl<'a> Len for &Cow<'a, str> {
    #[inline(always)]
    fn len(&self) -> usize {
        Len::len(*self)
    }
}

impl<'a> Len for &mut Cow<'a, str> {
    #[inline(always)]
    fn len(&self) -> usize {
        Len::len(*self)
    }
}

impl<'a, T> Len for Cow<'a, [T]>
where
    [T]: ToOwned,
{
    #[inline(always)]
    fn len(&self) -> usize {
        <[T]>::len(self)
    }
}

impl<'a, T> Len for &Cow<'a, [T]>
where
    [T]: ToOwned,
{
    #[inline(always)]
    fn len(&self) -> usize {
        Len::len(*self)
    }
}

impl<'a, T> Len for &mut Cow<'a, [T]>
where
    [T]: ToOwned,
{
    #[inline(always)]
    fn len(&self) -> usize {
        Len::len(*self)
    }
}

impl<T> Len for VecDeque<T> {
    #[inline(always)]
    fn len(&self) -> usize {
        <VecDeque<T>>::len(self)
    }
}

impl<T> Len for &VecDeque<T> {
    #[inline(always)]
    fn len(&self) -> usize {
        Len::len(*self)
    }
}

impl<T> Len for &mut VecDeque<T> {
    #[inline(always)]
    fn len(&self) -> usize {
        Len::len(&**self)
    }
}

impl<T> Len for LinkedList<T> {
    #[inline(always)]
    fn len(&self) -> usize {
        <LinkedList<T>>::len(self)
    }
}

impl<T> Len for &LinkedList<T> {
    #[inline(always)]
    fn len(&self) -> usize {
        Len::len(*self)
    }
}

impl<T> Len for &mut LinkedList<T> {
    #[inline(always)]
    fn len(&self) -> usize {
        Len::len(&**self)
    }
}
