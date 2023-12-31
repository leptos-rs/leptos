use super::Res;
use crate::error::ServerFnError;
use actix_web::{http::header, http::StatusCode, HttpResponse};
use bytes::Bytes;
use futures::Stream;
use send_wrapper::SendWrapper;
use std::fmt::Display;

pub struct ActixResponse(pub(crate) SendWrapper<HttpResponse>);

impl ActixResponse {
    pub fn into_inner(self) -> HttpResponse {
        self.0.take()
    }
}

impl<CustErr> Res<CustErr> for ActixResponse
where
    CustErr: Display,
{
    fn try_from_string(content_type: &str, data: String) -> Result<Self, ServerFnError<CustErr>> {
        let mut builder = HttpResponse::build(StatusCode::OK);
        Ok(ActixResponse(SendWrapper::new(
            builder
                .insert_header((header::CONTENT_TYPE, content_type))
                .body(data),
        )))
    }

    fn try_from_bytes(content_type: &str, data: Bytes) -> Result<Self, ServerFnError<CustErr>> {
        let mut builder = HttpResponse::build(StatusCode::OK);
        Ok(ActixResponse(SendWrapper::new(
            builder
                .insert_header((header::CONTENT_TYPE, content_type))
                .body(data),
        )))
    }

    fn error_response(err: ServerFnError<CustErr>) -> Self {
        ActixResponse(SendWrapper::new(
            HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR).body(err.to_string()),
        ))
    }

    fn try_from_stream(
        content_type: &str,
        data: impl Stream<Item = Result<Bytes, ServerFnError<CustErr>>>,
    ) -> Result<Self, ServerFnError<CustErr>> {
        todo!()
    }
}
