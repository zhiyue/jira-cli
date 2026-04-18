mod common;

use common::{in_blocking, spawn_mock_basic};
use jira_cli::config::{AuthConfig, JiraConfig};
use jira_cli::error::{Error, FieldError};
use jira_cli::field_resolver::{auto_rename_map, FieldResolver};
use serde_json::json;
use url::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

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

// ── auto_rename_map integration tests ────────────────────────────────────────

fn make_test_cfg(server: &MockServer) -> JiraConfig {
    JiraConfig {
        base_url: Url::parse(&server.uri()).unwrap(),
        auth: AuthConfig::Basic {
            user: "u".into(),
            password: "p".into(),
        },
        timeout_secs: 5,
        insecure: false,
        concurrency: 4,
        field_aliases: Default::default(),
        defaults: Default::default(),
        field_renames: Default::default(),
        jql_aliases: Default::default(),
        effective_renames_cache: Default::default(),
    }
}

#[tokio::test]
async fn auto_rename_excludes_collisions() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/field"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"id":"customfield_10006","name":"Story Points","custom":true,"clauseNames":[]},
            {"id":"customfield_11322","name":"Story Points","custom":true,"clauseNames":[]},
            {"id":"customfield_10000","name":"Epic Link","custom":true,"clauseNames":[]},
            {"id":"customfield_10800","name":"开发","custom":true,"clauseNames":[]},
            {"id":"summary","name":"Summary","custom":false,"clauseNames":[]}
        ])))
        .mount(&server)
        .await;

    in_blocking(move || {
        let map = auto_rename_map(&client).unwrap();
        // Epic Link: unique → included
        assert_eq!(map.get("customfield_10000"), Some(&"epic_link".to_string()));
        // Story Points has 2 candidates → both excluded
        assert!(!map.contains_key("customfield_10006"));
        assert!(!map.contains_key("customfield_11322"));
        // "开发" has empty slug → excluded
        assert!(!map.contains_key("customfield_10800"));
        // Non-custom fields never included
        assert!(!map.contains_key("summary"));
    })
    .await;
}

#[tokio::test]
async fn effective_renames_merges_manual_over_auto() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/field"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"id":"customfield_10006","name":"Story Points","custom":true,"clauseNames":[]}
        ])))
        .mount(&server)
        .await;

    // Emulate config where auto is on AND there's a manual override
    let mut cfg = make_test_cfg(&server);
    cfg.defaults.auto_rename_custom_fields = true;
    cfg.field_renames
        .insert("customfield_10006".into(), "points".into());

    in_blocking(move || {
        let map = cfg.effective_renames(&client).unwrap();
        // Manual wins over auto-generated "story_points"
        assert_eq!(map.get("customfield_10006"), Some(&"points".to_string()));
    })
    .await;
}

#[tokio::test]
async fn effective_renames_caches_auto_result() {
    let (server, client) = spawn_mock_basic().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/field"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"id":"customfield_10006","name":"Story Points","custom":true,"clauseNames":[]}
        ])))
        .expect(1) // Only ONE call despite two effective_renames invocations
        .mount(&server)
        .await;

    let mut cfg = make_test_cfg(&server);
    cfg.defaults.auto_rename_custom_fields = true;

    in_blocking(move || {
        let r1 = cfg.effective_renames(&client).unwrap();
        let r2 = cfg.effective_renames(&client).unwrap();
        assert_eq!(r1, r2);
        assert_eq!(
            r1.get("customfield_10006"),
            Some(&"story_points".to_string())
        );
    })
    .await;
}
