mod common;

use common::{in_blocking, spawn_mock_basic};
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn sends_basic_auth_header() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/serverInfo"))
        .and(header(
            "authorization",
            "Basic YWxpY2U6c2VjcmV0", // base64("alice:secret")
        ))
        .and(header("accept", "application/json"))
        .and(header("x-atlassian-token", "no-check"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "version": "8.13.5",
            "buildNumber": 813005
        })))
        .mount(&server)
        .await;

    in_blocking(move || {
        let resp = client
            .get_json::<serde_json::Value>("/rest/api/2/serverInfo")
            .expect("ok");
        assert_eq!(resp["version"], "8.13.5");
    })
    .await;
}
