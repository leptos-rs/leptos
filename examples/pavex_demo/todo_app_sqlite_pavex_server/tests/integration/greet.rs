use crate::helpers::TestApi;
use pavex::http::StatusCode;

#[tokio::test]
async fn greet_happy_path() {
    let api = TestApi::spawn().await;
    let name = "Ursula";

    let response = api
        .api_client
        .get(&format!("{}/api/greet/{name}", &api.api_address))
        .header("User-Agent", "Test runner")
        .send()
        .await
        .expect("Failed to execute request.");
    assert_eq!(response.status().as_u16(), StatusCode::OK.as_u16());
    assert_eq!(response.text().await.unwrap(), "Hello, Ursula!");
}

#[tokio::test]
async fn non_utf8_agent_is_rejected() {
    let api = TestApi::spawn().await;
    let name = "Ursula";

    let response = api
        .api_client
        .get(&format!("{}/api/greet/{name}", &api.api_address))
        .header("User-Agent", b"hello\xfa".as_slice())
        .send()
        .await
        .expect("Failed to execute request.");
    assert_eq!(response.status().as_u16(), StatusCode::BAD_REQUEST.as_u16());
    assert_eq!(
        response.text().await.unwrap(),
        "The `User-Agent` header must be a valid UTF-8 string"
    );
}
