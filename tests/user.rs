mod common;

use common::{in_blocking, spawn_mock_basic};
use jira_cli::api::user;
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn get_user_by_name() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/user"))
        .and(query_param("username", "alice"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"name":"alice"})))
        .mount(&server)
        .await;

    in_blocking(move || {
        let v = user::get(&client, "alice").unwrap();
        assert_eq!(v["name"], "alice");
    })
    .await;
}

#[tokio::test]
async fn search_users() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/user/search"))
        .and(query_param("username", "ali"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"name":"alice"}])))
        .mount(&server)
        .await;

    in_blocking(move || {
        let v = user::search(&client, "ali").unwrap();
        assert_eq!(v.len(), 1);
    })
    .await;
}
