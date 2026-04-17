mod common;

use common::{in_blocking, spawn_mock_cookie};
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn cookie_header_is_sent() {
    let (server, client) = spawn_mock_cookie().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/myself"))
        .and(header("cookie", "JSESSIONID=abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"name": "alice"})))
        .mount(&server)
        .await;

    in_blocking(move || {
        let v: serde_json::Value = client.get_json("/rest/api/2/myself").unwrap();
        assert_eq!(v["name"], "alice");
    })
    .await;
}

#[tokio::test]
async fn captcha_header_surfaces_as_auth_error() {
    use jira_cli::error::{AuthError, Error};
    let (server, client) = spawn_mock_cookie().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/myself"))
        .respond_with(ResponseTemplate::new(401).insert_header(
            "X-Seraph-LoginReason",
            "AUTHENTICATION_DENIED_CAPTCHA_REQUIRED",
        ))
        .mount(&server)
        .await;

    in_blocking(move || {
        let err = client
            .get_json::<serde_json::Value>("/rest/api/2/myself")
            .unwrap_err();
        assert!(
            matches!(err, Error::Auth(AuthError::CaptchaRequired)),
            "expected CaptchaRequired, got {err:?}"
        );
    })
    .await;
}

#[tokio::test]
async fn authentication_denied_maps_to_unauthorized() {
    use jira_cli::error::{AuthError, Error};
    let (server, client) = spawn_mock_cookie().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/myself"))
        .respond_with(
            ResponseTemplate::new(401)
                .insert_header("X-Seraph-LoginReason", "AUTHENTICATION_DENIED"),
        )
        .mount(&server)
        .await;

    in_blocking(move || {
        let err = client
            .get_json::<serde_json::Value>("/rest/api/2/myself")
            .unwrap_err();
        assert!(matches!(err, Error::Auth(AuthError::Unauthorized)));
    })
    .await;
}
