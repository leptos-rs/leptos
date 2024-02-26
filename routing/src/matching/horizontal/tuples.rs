use super::{PartialPathMatch, PathSegment, PossibleRouteMatch};
use alloc::{string::String, vec::Vec};

macro_rules! tuples {
    ($($ty:ident),*) => {
        impl<$($ty),*> PossibleRouteMatch for ($($ty,)*)
        where
			$($ty: PossibleRouteMatch),*,
        {
            fn matches<'a>(&self, path: &'a str) -> Option<&'a str>
            {
                #[allow(non_snake_case)]
                let ($($ty,)*) = &self;
                $(let path = $ty.matches(path)?;)*
                Some(path)
            }

            fn test<'a>(&self, path: &'a str) -> Option<PartialPathMatch<'a>>
            {
				let mut full_params = Vec::new();
                let mut matched_len = 0;
                #[allow(non_snake_case)]
                let ($($ty,)*) = &self;
                let remaining = path;
                $(
                    let PartialPathMatch {
                        remaining,
                        matched,
                        params
                    } = $ty.test(remaining)?;
                    matched_len += matched.len();
                    full_params.extend(params);
                )*
                Some(PartialPathMatch {
                    remaining,
                    matched: &path[0..matched_len],
                    params: full_params
                })
            }

            fn generate_path(&self, path: &mut Vec<PathSegment>) {
                #[allow(non_snake_case)]
                let ($($ty,)*) = &self;
                $(
                    $ty.generate_path(path);
                )*
            }
        }
	};
}

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
