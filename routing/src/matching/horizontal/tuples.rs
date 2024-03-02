use super::{PartialPathMatch, PathSegment, PossibleRouteMatch};
use core::iter::Chain;

macro_rules! chain_types {
    ($first:ty, $second:ty, ) => {
        Chain<
            $first,
            <<$second as PossibleRouteMatch>::ParamsIter<'a> as IntoIterator>::IntoIter
        >
    };
    ($first:ty, $second:ty, $($rest:ty,)+) => {
        chain_types!(
            Chain<
                $first,
                <<$second as PossibleRouteMatch>::ParamsIter<'a> as IntoIterator>::IntoIter,
            >,
            $($rest,)+
        )
    }
}

macro_rules! tuples {
    ($first:ident => $($ty:ident),*) => {
        impl<$first, $($ty),*> PossibleRouteMatch for ($first, $($ty,)*)
        where
            Self: core::fmt::Debug,
            $first: PossibleRouteMatch,
			$($ty: PossibleRouteMatch),*,
        {
            type ParamsIter<'a> = chain_types!(<<$first>::ParamsIter<'a> as IntoIterator>::IntoIter, $($ty,)*);

            fn test<'a>(&self, path: &'a str) -> Option<PartialPathMatch<'a, Self::ParamsIter<'a>>> {
                let mut matched_len = 0;
                #[allow(non_snake_case)]
                let ($first, $($ty,)*) = &self;
                let remaining = path;
                let PartialPathMatch {
                    remaining,
                    matched,
                    params
                } = $first.test(remaining)?;
                matched_len += matched.len();
                let params_iter = params.into_iter();
                $(
                    let PartialPathMatch {
                        remaining,
                        matched,
                        params
                    } = $ty.test(remaining)?;
                    matched_len += matched.len();
                    let params_iter = params_iter.chain(params);
                )*
                Some(PartialPathMatch {
                    remaining,
                    matched: &path[0..matched_len],
                    params: params_iter
                })
            }

            fn generate_path(&self, path: &mut Vec<PathSegment>) {
                #[allow(non_snake_case)]
                let ($first, $($ty,)*) = &self;
                $first.generate_path(path);
                $(
                    $ty.generate_path(path);
                )*
            }
        }
	};
}

tuples!(A => B);
tuples!(A => B, C);
tuples!(A => B, C, D);
tuples!(A => B, C, D, E);
tuples!(A => B, C, D, E, F);
tuples!(A => B, C, D, E, F, G);
tuples!(A => B, C, D, E, F, G, H);
tuples!(A => B, C, D, E, F, G, H, I);
tuples!(A => B, C, D, E, F, G, H, I, J);
tuples!(A => B, C, D, E, F, G, H, I, J, K);
tuples!(A => B, C, D, E, F, G, H, I, J, K, L);
tuples!(A => B, C, D, E, F, G, H, I, J, K, L, M);
tuples!(A => B, C, D, E, F, G, H, I, J, K, L, M, N);
tuples!(A => B, C, D, E, F, G, H, I, J, K, L, M, N, O);
tuples!(A => B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
tuples!(A => B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q);
tuples!(A => B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R);
tuples!(A => B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S);
tuples!(A => B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T);
tuples!(A => B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U);
tuples!(A => B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V);
tuples!(A => B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W);
tuples!(A => B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X);
/*tuples!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y
);
tuples!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y,
    Z
);*/
