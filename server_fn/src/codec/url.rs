use super::{Encoding, FromReq, IntoReq};
use crate::{
    error::ServerFnError,
    request::{ClientReq, Req},
};
use http::Method;
use serde::{de::DeserializeOwned, Serialize};

/// Pass arguments as a URL-encoded query string of a `GET` request.
pub struct GetUrl;

/// Pass arguments as the URL-encoded body of a `POST` request.
pub struct PostUrl;

impl Encoding for GetUrl {
    const CONTENT_TYPE: &'static str = "application/x-www-form-urlencoded";
    const METHOD: Method = Method::GET;
}

impl<CustErr, T, Request> IntoReq<GetUrl, Request, CustErr> for T
where
    Request: ClientReq<CustErr>,
    T: Serialize + Send,
{
    fn into_req(
        self,
        path: &str,
        accepts: &str,
    ) -> Result<Request, ServerFnError<CustErr>> {
        let data = serde_qs::to_string(&self)
            .map_err(|e| ServerFnError::Serialization(e.to_string()))?;
        Request::try_new_get(path, accepts, GetUrl::CONTENT_TYPE, &data)
    }
}

impl<CustErr, T, Request> FromReq<GetUrl, Request, CustErr> for T
where
    Request: Req<CustErr> + Send + 'static,
    T: DeserializeOwned,
{
    async fn from_req(req: Request) -> Result<Self, ServerFnError<CustErr>> {
        let string_data = req.as_query().unwrap_or_default();
        let args = serde_qs::Config::new(5, false)
            .deserialize_str::<Self>(string_data)
            .map_err(|e| ServerFnError::Args(e.to_string()))?;
        Ok(args)
    }
}

impl Encoding for PostUrl {
    const CONTENT_TYPE: &'static str = "application/x-www-form-urlencoded";
    const METHOD: Method = Method::POST;
}

impl<CustErr, T, Request> IntoReq<PostUrl, Request, CustErr> for T
where
    Request: ClientReq<CustErr>,
    T: Serialize + Send,
{
    fn into_req(
        self,
        path: &str,
        accepts: &str,
    ) -> Result<Request, ServerFnError<CustErr>> {
        let qs = serde_qs::to_string(&self)
            .map_err(|e| ServerFnError::Serialization(e.to_string()))?;
        Request::try_new_post(path, accepts, PostUrl::CONTENT_TYPE, qs)
    }
}

impl<CustErr, T, Request> FromReq<PostUrl, Request, CustErr> for T
where
    Request: Req<CustErr> + Send + 'static,
    T: DeserializeOwned,
{
    async fn from_req(req: Request) -> Result<Self, ServerFnError<CustErr>> {
        let string_data = req.try_into_string().await?;
        let args = serde_qs::Config::new(5, false)
            .deserialize_str::<Self>(&string_data)
            .map_err(|e| ServerFnError::Args(e.to_string()))?;
        Ok(args)
    }
}

/* #[async_trait]
impl<T, Request, Response> Codec<Request, Response, GetUrlJson> for T
where
    T: DeserializeOwned + Serialize + Send,
    Request: Req<CustErr> + Send,
    Response: Res<CustErr> + Send,
{
    async fn from_req(req: Request) -> Result<Self, ServerFnError<CustErr>> {
        let string_data = req.try_into_string()?;

        let args = serde_json::from_str::<Self>(&string_data)
            .map_err(|e| ServerFnError::Args(e.to_string()))?;
        Ok(args)
    }

    async fn into_req(self) -> Result<Request, ServerFnError<CustErr>> {
        /* let qs = serde_qs::to_string(&self)?;
        let req = http::Request::builder()
            .method("GET")
            .header(
                http::header::CONTENT_TYPE,
                <GetUrlJson as Encoding>::REQUEST_CONTENT_TYPE,
            )
            .body(Body::from(qs))?;
        Ok(req) */
        todo!()
    }

    async fn from_res(res: Response) -> Result<Self, ServerFnError<CustErr>> {
        todo!()
        /* let (_parts, body) = res.into_parts();

        let body_bytes = body
            .collect()
            .await
            .map(|c| c.to_bytes())
            .map_err(|e| ServerFnError::Deserialization(e.to_string()))?;
        let string_data = String::from_utf8(body_bytes.to_vec())?;
        serde_json::from_str(&string_data)
            .map_err(|e| ServerFnError::Deserialization(e.to_string())) */
    }

    async fn into_res(self) -> Response {
        // Need to catch and err or here, or handle Errors at a higher level
        let data = match serde_json::to_string(&self) {
            Ok(d) => d,
            Err(e) => return e.into_err_res(),
        };
        let builder = http::Response::builder();
        let res = builder
            .status(200)
            .header(
                http::header::CONTENT_TYPE,
                <GetUrlJson as Encoding>::RESPONSE_CONTENT_TYPE,
            )
            .body(Body::from(data))
            .unwrap();
        res
    }
}
 */
