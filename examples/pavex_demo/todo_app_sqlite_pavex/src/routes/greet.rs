use crate::user_agent::UserAgent;
use pavex::{request::route::RouteParams, response::Response};

#[RouteParams]
pub struct GreetParams {
    pub name: String,
}
pub fn greet(
    params: RouteParams<GreetParams>,
    user_agent: UserAgent,
) -> Response {
    if let UserAgent::Unknown = user_agent {
        return Response::unauthorized()
            .set_typed_body("You must provide a `User-Agent` header")
            .box_body();
    }
    let GreetParams { name } = params.0;
    Response::ok()
        .set_typed_body(format!("Hello, {name}!"))
        .box_body()
}
