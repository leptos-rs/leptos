use std::{
    borrow::Cow,
    collections::{LinkedList, VecDeque},
};

/// A trait for getting the length of a collection.
pub trait Len {
    /// Returns the length of the collection.
    fn len(&self) -> usize;

    /// Returns true if the collection is empty
    #[inline(always)]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

macro_rules! delegate_impl_len {
    (<$($lt: lifetime,)*$($generics: ident,)*> $ty:ty) => {
        impl<$($lt,)*$($generics,)*> Len for $ty {
            #[inline(always)]
            fn len(&self) -> usize {
                <$ty>::len(self)
            }

            #[inline(always)]
            fn is_empty(&self) -> bool {
                <$ty>::is_empty(self)
            }
        }

        impl<$($lt,)*$($generics,)*> Len for &$ty {
            #[inline(always)]
            fn len(&self) -> usize {
                Len::len(*self)
            }

            #[inline(always)]
            fn is_empty(&self) -> bool {
                Len::is_empty(*self)
            }
        }

        impl<$($lt,)*$($generics,)*> Len for &mut $ty {
            #[inline(always)]
            fn len(&self) -> usize {
                Len::len(*self)
            }

            #[inline(always)]
            fn is_empty(&self) -> bool {
                Len::is_empty(*self)
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

impl Len for Cow<'_, str> {
    #[inline(always)]
    fn len(&self) -> usize {
        <str>::len(self)
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        <str>::is_empty(self)
    }
}

impl Len for &Cow<'_, str> {
    #[inline(always)]
    fn len(&self) -> usize {
        Len::len(*self)
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        Len::is_empty(*self)
    }
}

impl Len for &mut Cow<'_, str> {
    #[inline(always)]
    fn len(&self) -> usize {
        Len::len(*self)
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        Len::is_empty(*self)
    }
}

impl<T> Len for Cow<'_, [T]>
where
    [T]: ToOwned,
{
    #[inline(always)]
    fn len(&self) -> usize {
        <[T]>::len(self)
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        <[T]>::is_empty(self)
    }
}

impl<T> Len for &Cow<'_, [T]>
where
    [T]: ToOwned,
{
    #[inline(always)]
    fn len(&self) -> usize {
        Len::len(*self)
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        Len::is_empty(*self)
    }
}

impl<T> Len for &mut Cow<'_, [T]>
where
    [T]: ToOwned,
{
    #[inline(always)]
    fn len(&self) -> usize {
        Len::len(*self)
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        Len::is_empty(*self)
    }
}

impl<T> Len for VecDeque<T> {
    #[inline(always)]
    fn len(&self) -> usize {
        <VecDeque<T>>::len(self)
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        <VecDeque<T>>::is_empty(self)
    }
}

impl<T> Len for &VecDeque<T> {
    #[inline(always)]
    fn len(&self) -> usize {
        Len::len(*self)
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        Len::is_empty(*self)
    }
}

impl<T> Len for &mut VecDeque<T> {
    #[inline(always)]
    fn len(&self) -> usize {
        Len::len(&**self)
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        Len::is_empty(*self)
    }
}

impl<T> Len for LinkedList<T> {
    #[inline(always)]
    fn len(&self) -> usize {
        <LinkedList<T>>::len(self)
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        <LinkedList<T>>::is_empty(self)
    }
}

impl<T> Len for &LinkedList<T> {
    #[inline(always)]
    fn len(&self) -> usize {
        Len::len(*self)
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        Len::is_empty(*self)
    }
}

impl<T> Len for &mut LinkedList<T> {
    #[inline(always)]
    fn len(&self) -> usize {
        Len::len(&**self)
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        Len::is_empty(*self)
    }
}
