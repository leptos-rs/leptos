use leptos::prelude::*;
use leptos_router::{LazyRoute, lazy_route};

struct MutRoute {
    value: u32,
}

#[lazy_route]
impl LazyRoute for MutRoute {
    fn data() -> Self {
        Self { value: 41 }
    }

    // `mut this` is a binding-mode pattern, which is not valid in expression
    // position. The macro must forward the argument through a fresh binding
    // rather than splicing this pattern into the generated call expression.
    fn view(mut this: Self) -> AnyView {
        this.value += 1;
        let _ = this.value;
        ().into_any()
    }
}

struct DestructuredRoute {
    value: u32,
}

#[lazy_route]
impl LazyRoute for DestructuredRoute {
    fn data() -> Self {
        Self { value: 0 }
    }

    // A struct-destructuring pattern is likewise invalid as an expression.
    fn view(DestructuredRoute { value }: Self) -> AnyView {
        let _ = value;
        ().into_any()
    }
}

#[test]
fn lazy_route_data_constructs() {
    assert_eq!(MutRoute::data().value, 41);
    assert_eq!(DestructuredRoute::data().value, 0);
}
