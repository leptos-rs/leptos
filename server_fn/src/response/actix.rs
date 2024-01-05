use super::Res;
use crate::error::{ServerFnError, ServerFnErrorErr};
use actix_web::{
    http::{header, StatusCode},
    HttpResponse,
};
use bytes::Bytes;
use futures::{Stream, StreamExt};
use send_wrapper::SendWrapper;
use std::fmt::{Debug, Display};

pub struct ActixResponse(pub(crate) SendWrapper<HttpResponse>);

impl ActixResponse {
    pub fn take(self) -> HttpResponse {
        self.0.take()
    }
}

impl From<HttpResponse> for ActixResponse {
    fn from(value: HttpResponse) -> Self {
        Self(SendWrapper::new(value))
    }
}

impl<CustErr> Res<CustErr> for ActixResponse
where
    CustErr: Display + Debug + 'static,
{
    fn try_from_string(
        content_type: &str,
        data: String,
    ) -> Result<Self, ServerFnError<CustErr>> {
        let mut builder = HttpResponse::build(StatusCode::OK);
        Ok(ActixResponse(SendWrapper::new(
            builder
                .insert_header((header::CONTENT_TYPE, content_type))
                .body(data),
        )))
    }

    fn try_from_bytes(
        content_type: &str,
        data: Bytes,
    ) -> Result<Self, ServerFnError<CustErr>> {
        let mut builder = HttpResponse::build(StatusCode::OK);
        Ok(ActixResponse(SendWrapper::new(
            builder
                .insert_header((header::CONTENT_TYPE, content_type))
                .body(data),
        )))
    }

    fn error_response(err: ServerFnError<CustErr>) -> Self {
        ActixResponse(SendWrapper::new(
            HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
                .body(err.to_string()),
        ))
    }

    fn try_from_stream(
        content_type: &str,
        data: impl Stream<Item = Result<Bytes, ServerFnError<CustErr>>> + 'static,
    ) -> Result<Self, ServerFnError<CustErr>> {
        let mut builder = HttpResponse::build(StatusCode::OK);
        Ok(ActixResponse(SendWrapper::new(
            builder
                .insert_header((header::CONTENT_TYPE, content_type))
                .streaming(
                    data.map(|data| data.map_err(ServerFnErrorErr::from)),
                ),
        )))
    }
}
