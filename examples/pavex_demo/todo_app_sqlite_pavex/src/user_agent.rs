use pavex::{
    http::header::{ToStrError, USER_AGENT},
    request::RequestHead,
    response::Response,
};

pub enum UserAgent {
    /// No User-Agent header was provided
    Unknown,
    /// The value of the 'User-Agent' header for the incoming request
    Known(String),
}
impl UserAgent {
    pub fn extract(request_head: &RequestHead) -> Result<Self, ToStrError> {
        let Some(user_agent) = request_head.headers.get(USER_AGENT) else {
            return Ok(UserAgent::Unknown);
        };

        user_agent.to_str().map(|s| UserAgent::Known(s.into()))
    }
}

pub fn invalid_user_agent(_e: &ToStrError) -> Response {
    Response::bad_request()
        .set_typed_body("The `User-Agent` header must be a valid UTF-8 string")
        .box_body()
}
