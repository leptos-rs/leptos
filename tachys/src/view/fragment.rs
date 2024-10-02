use super::any_view::{AnyView, IntoAny};

/// A typed-erased collection of different views.
pub struct Fragment {
    /// The nodes contained in the fragment.
    pub nodes: Vec<AnyView>,
}

/// Converts some view into a type-erased collection of views.
pub trait IntoFragment {
    /// Converts some view into a type-erased collection of views.
    fn into_fragment(self) -> Fragment;
}

impl FromIterator<AnyView> for Fragment {
    fn from_iter<T: IntoIterator<Item = AnyView>>(iter: T) -> Self {
        Fragment::new(iter.into_iter().collect())
    }
}

impl From<AnyView> for Fragment {
    fn from(view: AnyView) -> Self {
        Fragment::new(vec![view])
    }
}

impl From<Fragment> for AnyView {
    fn from(value: Fragment) -> Self {
        value.nodes.into_any()
    }
}

impl Fragment {
    /// Creates a new [`Fragment`].
    #[inline(always)]
    pub fn new(nodes: Vec<AnyView>) -> Self {
        Self { nodes }
    }
}

impl<T> IntoFragment for Vec<T>
where
    T: IntoAny,
{
    fn into_fragment(self) -> Fragment {
        Fragment::new(self.into_iter().map(IntoAny::into_any).collect())
    }
}

impl<const N: usize, T> IntoFragment for [T; N]
where
    T: IntoAny,
{
    fn into_fragment(self) -> Fragment {
        Fragment::new(self.into_iter().map(IntoAny::into_any).collect())
    }
}

macro_rules! tuples {
	($($ty:ident),*) => {
		impl<$($ty),*> IntoFragment for ($($ty,)*)
		where
			$($ty: IntoAny),*,

		{
            fn into_fragment(self) -> Fragment {
                #[allow(non_snake_case)]
			    let ($($ty,)*) = self;
                Fragment::new(vec![$($ty.into_any(),)*])
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
tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W);
tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X);
tuples!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y
);
tuples!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y,
    Z
);
