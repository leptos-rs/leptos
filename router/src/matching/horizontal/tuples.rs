use super::{PartialPathMatch, PathSegment, PossibleRouteMatch};

macro_rules! tuples {
    ($first:ident => $($ty:ident),*) => {
        impl<$first, $($ty),*> PossibleRouteMatch for ($first, $($ty,)*)
        where
            Self: core::fmt::Debug,
            $first: PossibleRouteMatch,
			$($ty: PossibleRouteMatch),*,
        {
            fn optional(&self) -> bool {
                #[allow(non_snake_case)]
                let ($first, $($ty,)*) = &self;
                [$first.optional(), $($ty.optional()),*].into_iter().any(|n| n)
            }

            fn test<'a>(&self, path: &'a str) -> Option<PartialPathMatch<'a>> {
                #[allow(non_snake_case)]
                let ($first, $($ty,)*) = &self;

                // on the first run, include all optionals
                let mut include_optionals = {
                    [$first.optional(), $($ty.optional()),*].into_iter().filter(|n| *n).count()
                };

                loop {
                    let mut nth_field = 0;
                    let mut matched_len = 0;
                    let mut r = path;

                    let mut p = Vec::new();
                    let mut m = String::new();

                    if $first.optional() {
                        nth_field += 1;
                    }
                    if !$first.optional() || nth_field <= include_optionals {
                        match $first.test(r) {
                            None => {
                                return None;
                            },
                            Some(PartialPathMatch { remaining, matched, params }) => {
                                p.extend(params.into_iter());
                                m.push_str(matched);
                                r = remaining;
                            },
                        }
                    }

                    matched_len += m.len();
                    $(
                        if $ty.optional() {
                            nth_field += 1;
                        }
                        if !$ty.optional() || nth_field <= include_optionals {
                            let PartialPathMatch {
                                remaining,
                                matched,
                                params
                            } = match $ty.test(r) {
                                None => if $ty.optional() {
                                    return None;
                                } else {
                                    if include_optionals == 0 {
                                        return None;
                                    }
                                    include_optionals -= 1;
                                    continue;
                                },
                                Some(v) => v,
                            };
                            r = remaining;
                            matched_len += matched.len();
                            p.extend(params);
                        }
                    )*
                    return Some(PartialPathMatch {
                        remaining: r,
                        matched: &path[0..matched_len],
                        params: p
                    });
                }
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

impl<A> PossibleRouteMatch for (A,)
where
    Self: core::fmt::Debug,
    A: PossibleRouteMatch,
{
    fn optional(&self) -> bool {
        self.0.optional()
    }

    fn test<'a>(&self, path: &'a str) -> Option<PartialPathMatch<'a>> {
        let remaining = path;
        let PartialPathMatch {
            remaining,
            matched,
            params,
        } = self.0.test(remaining)?;
        Some(PartialPathMatch {
            remaining,
            matched: &path[0..matched.len()],
            params,
        })
    }

    fn generate_path(&self, path: &mut Vec<PathSegment>) {
        self.0.generate_path(path);
    }
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
