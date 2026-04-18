mod common;

use common::{in_blocking, spawn_mock_basic};
use jira_cli::error::{Error, FieldError};
use jira_cli::field_resolver::FieldResolver;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn resolve_unique_name() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/field"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"id":"customfield_10020","name":"Story Points","custom":true,"clauseNames":[]}
        ])))
        .mount(&server)
        .await;

    in_blocking(move || {
        let r = FieldResolver::new(&client);
        let id = r.resolve("Story Points").unwrap();
        assert_eq!(id, "customfield_10020");
    })
    .await;
}

#[tokio::test]
async fn customfield_passthrough() {
    let (_server, client) = spawn_mock_basic().await;
    // no mock needed — resolver short-circuits for customfield_*
    in_blocking(move || {
        let r = FieldResolver::new(&client);
        let id = r.resolve("customfield_10030").unwrap();
        assert_eq!(id, "customfield_10030");
    })
    .await;
}

#[tokio::test]
async fn ambiguous_name_errors() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/field"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"id":"customfield_10020","name":"Story Points","custom":true,"clauseNames":[]},
            {"id":"customfield_10021","name":"Story Points","custom":true,"clauseNames":[]}
        ])))
        .mount(&server)
        .await;

    in_blocking(move || {
        let r = FieldResolver::new(&client);
        let err = r.resolve("Story Points").unwrap_err();
        match err {
            Error::FieldResolve(FieldError::Ambiguous { candidates, .. }) => {
                assert_eq!(candidates.len(), 2);
            }
            other => panic!("expected Ambiguous, got {other:?}"),
        }
    })
    .await;
}

#[tokio::test]
async fn unknown_name_errors() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/field"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"id":"summary","name":"Summary","custom":false,"clauseNames":[]}
        ])))
        .mount(&server)
        .await;

    in_blocking(move || {
        let r = FieldResolver::new(&client);
        let err = r.resolve("Foo Bar").unwrap_err();
        assert!(matches!(err, Error::FieldResolve(FieldError::Unknown(_))));
    })
    .await;
}

#[tokio::test]
async fn alias_takes_precedence_over_ambiguous_auto_resolution() {
    let (server, client) = spawn_mock_basic().await;
    // Two customfields share the same name — without an alias this would be ambiguous.
    Mock::given(method("GET"))
        .and(path("/rest/api/2/field"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"id":"customfield_11322","name":"Story Points","custom":true,"clauseNames":[]},
            {"id":"customfield_10006","name":"Story Points","custom":true,"clauseNames":[]}
        ])))
        .mount(&server)
        .await;

    in_blocking(move || {
        use std::collections::HashMap;
        let mut aliases = HashMap::new();
        aliases.insert("Story Points".into(), "customfield_10006".into());
        let r = FieldResolver::new(&client).with_aliases(aliases);
        let id = r.resolve("Story Points").unwrap();
        assert_eq!(id, "customfield_10006");
    })
    .await;
}

#[tokio::test]
async fn alias_overrides_unique_auto_resolution() {
    // Even when the name is unique in /field, alias still wins.
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/field"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"id":"customfield_10020","name":"Story Points","custom":true,"clauseNames":[]}
        ])))
        .mount(&server)
        .await;

    in_blocking(move || {
        use std::collections::HashMap;
        let mut aliases = HashMap::new();
        aliases.insert("Story Points".into(), "customfield_99999".into());
        let r = FieldResolver::new(&client).with_aliases(aliases);
        let id = r.resolve("Story Points").unwrap();
        assert_eq!(id, "customfield_99999");
    })
    .await;
}
