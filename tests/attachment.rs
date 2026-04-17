mod common;

use common::{in_blocking, spawn_mock_basic};
use jira_cli::api::attachment;
use serde_json::json;
use tempfile::NamedTempFile;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn upload_sends_multipart_with_no_check_token() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("POST"))
        .and(path("/rest/api/2/issue/MGX-1/attachments"))
        .and(header("x-atlassian-token", "no-check"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"id":"20000","filename":"hello.txt"}
        ])))
        .mount(&server)
        .await;

    in_blocking(move || {
        let mut f = NamedTempFile::new().unwrap();
        use std::io::Write;
        writeln!(f, "hello world").unwrap();
        let v = attachment::upload(&client, "MGX-1", &[f.path().to_path_buf()]).unwrap();
        let arr = v.as_array().unwrap();
        assert_eq!(arr[0]["id"], "20000");
    })
    .await;
}

#[tokio::test]
async fn delete_attachment() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("DELETE"))
        .and(path("/rest/api/2/attachment/20000"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    in_blocking(move || {
        attachment::delete(&client, "20000").unwrap();
    })
    .await;
}
