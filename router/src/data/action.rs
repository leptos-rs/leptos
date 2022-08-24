use std::{future::Future, rc::Rc};

use crate::{PinnedFuture, Request, Response};

#[derive(Clone)]
pub struct Action {
    f: Rc<dyn Fn(&Request) -> PinnedFuture<Response>>,
}

impl Action {
    pub async fn send(&self, req: &Request) -> Response {
        (self.f)(req).await
    }
}

impl<F, Fu> From<F> for Action
where
    F: Fn(&Request) -> Fu + Clone + 'static,
    Fu: Future<Output = Response> + 'static,
{
    fn from(f: F) -> Self {
        Self {
            f: Rc::new(move |req| Box::pin(f(req))),
        }
    }
}

impl std::fmt::Debug for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Action").finish()
    }
}
