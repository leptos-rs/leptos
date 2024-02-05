#![allow(non_snake_case)]

pub trait TupleBuilder<Next> {
    type Output;

    fn next_tuple(self, next: Next) -> Self::Output;
}

pub trait ConcatTuples<Next> {
    type Output;

    fn concat(self, next: Next) -> Self::Output;
}

macro_rules! impl_tuple_builder {
    ($($ty:ident),* => $last:ident) => {
		impl<$($ty),*, $last> TupleBuilder<$last> for ($($ty,)*) {
			type Output = ($($ty,)* $last);

			fn next_tuple(self, next: $last) -> Self::Output {
				let ($($ty,)*) = self;
				($($ty,)* next)
			}
		}
    };
}

impl<A> TupleBuilder<A> for () {
    type Output = (A,);

    fn next_tuple(self, next: A) -> Self::Output {
        (next,)
    }
}

impl_tuple_builder!(A => B);
impl_tuple_builder!(A, B => C);
impl_tuple_builder!(A, B, C => D);
impl_tuple_builder!(A, B, C, D => E);
impl_tuple_builder!(A, B, C, D, E => F);
impl_tuple_builder!(A, B, C, D, E, F => G);
impl_tuple_builder!(A, B, C, D, E, F, G => H);
impl_tuple_builder!(A, B, C, D, E, F, G, H => I);
impl_tuple_builder!(A, B, C, D, E, F, G, H, I => J);
impl_tuple_builder!(A, B, C, D, E, F, G, H, I, J => K);
impl_tuple_builder!(A, B, C, D, E, F, G, H, I, J, K => L);
impl_tuple_builder!(A, B, C, D, E, F, G, H, I, J, K, L => M);
impl_tuple_builder!(A, B, C, D, E, F, G, H, I, J, K, L, M => N);
impl_tuple_builder!(A, B, C, D, E, F, G, H, I, J, K, L, M, N => O);
impl_tuple_builder!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O => P);
impl_tuple_builder!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P => Q);
impl_tuple_builder!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q => R);
impl_tuple_builder!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R => S);
impl_tuple_builder!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S => T
);
impl_tuple_builder!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T => U
);
impl_tuple_builder!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U => V
);
impl_tuple_builder!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V => W
);
impl_tuple_builder!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W => X
);
impl_tuple_builder!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X => Y
);
impl_tuple_builder!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y =>
    Z
);
