mod common;

use common::{in_blocking, spawn_mock_basic};
use jira_cli::api::comment;
use serde_json::json;
use wiremock::matchers::{body_partial_json, method, path, query_param};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn list_comments() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/issue/MGX-1/comment"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "startAt": 0, "maxResults": 50, "total": 2,
            "comments": [
                {"id":"10010","body":"first","author":{"name":"alice"}},
                {"id":"10011","body":"second","author":{"name":"bob"}}
            ]
        })))
        .mount(&server)
        .await;

    in_blocking(move || {
        let page = comment::list(&client, "MGX-1").unwrap();
        assert_eq!(page.comments.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn add_comment() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("POST"))
        .and(path("/rest/api/2/issue/MGX-1/comment"))
        .and(body_partial_json(json!({"body": "hello"})))
        .respond_with(
            ResponseTemplate::new(201).set_body_json(json!({"id": "10020","body":"hello"})),
        )
        .mount(&server)
        .await;

    in_blocking(move || {
        let v = comment::add(&client, "MGX-1", "hello").unwrap();
        assert_eq!(v["id"], "10020");
    })
    .await;
}

#[tokio::test]
async fn update_comment() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("PUT"))
        .and(path("/rest/api/2/issue/MGX-1/comment/10020"))
        .and(body_partial_json(json!({"body": "edited"})))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"id":"10020","body":"edited"})),
        )
        .mount(&server)
        .await;

    in_blocking(move || {
        let v = comment::update(&client, "MGX-1", "10020", "edited").unwrap();
        assert_eq!(v["body"], "edited");
    })
    .await;
}

#[tokio::test]
async fn delete_comment() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("DELETE"))
        .and(path("/rest/api/2/issue/MGX-1/comment/10020"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    in_blocking(move || {
        comment::delete(&client, "MGX-1", "10020").unwrap();
    })
    .await;
}

#[tokio::test]
async fn list_paginates_multiple_pages() {
    let (server, client) = spawn_mock_basic().await;

    // Page 1: 50 comments, total=120
    let page1: Vec<_> = (0..50)
        .map(|i| json!({"id": format!("{}", 10000+i), "body": format!("c{i}")}))
        .collect();
    Mock::given(method("GET"))
        .and(path("/rest/api/2/issue/MGX-1/comment"))
        .and(query_param("startAt", "0"))
        .and(query_param("maxResults", "50"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "startAt": 0, "maxResults": 50, "total": 120, "comments": page1
        })))
        .expect(1)
        .mount(&server)
        .await;

    let page2: Vec<_> = (50..100)
        .map(|i| json!({"id": format!("{}", 10000+i), "body": format!("c{i}")}))
        .collect();
    Mock::given(method("GET"))
        .and(path("/rest/api/2/issue/MGX-1/comment"))
        .and(query_param("startAt", "50"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "startAt": 50, "maxResults": 50, "total": 120, "comments": page2
        })))
        .expect(1)
        .mount(&server)
        .await;

    let page3: Vec<_> = (100..120)
        .map(|i| json!({"id": format!("{}", 10000+i), "body": format!("c{i}")}))
        .collect();
    Mock::given(method("GET"))
        .and(path("/rest/api/2/issue/MGX-1/comment"))
        .and(query_param("startAt", "100"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "startAt": 100, "maxResults": 50, "total": 120, "comments": page3
        })))
        .expect(1)
        .mount(&server)
        .await;

    in_blocking(move || {
        use jira_cli::api::paging::PageParams;
        let params = PageParams {
            start_at: 0,
            page_size: 50,
            max: None,
        };
        let mut iter = comment::list_paged(&client, "MGX-1", params);
        let mut total = 0;
        for item in iter.by_ref() {
            item.unwrap();
            total += 1;
        }
        assert_eq!(total, 120);
        assert_eq!(iter.total(), Some(120));
    })
    .await;
}
