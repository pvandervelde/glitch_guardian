use super::*;
use axum::http::HeaderMap;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::sync::Arc;

// Helper function to create a mock AppState
fn create_mock_state() -> Arc<AppState> {
    Arc::new(AppState {
        octocrab: Octocrab::builder().build().unwrap(),
        project_id: "test_project_id".to_string(),
        webhook_secret: "test_webhook_secret".to_string(),
    })
}

#[test]
fn test_verify_github_signature_valid() {
    let secret = "test_secret";
    let body = r#"{"action": "opened", "issue": {"node_id": "test_node_id"}}"#;

    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body.as_bytes());
    let result = mac.finalize();
    let signature = format!("sha256={}", hex::encode(result.into_bytes()));

    let mut headers = HeaderMap::new();
    headers.insert("X-Hub-Signature-256", signature.parse().unwrap());

    assert!(verify_github_signature(secret, &headers, body));
}

#[test]
fn test_verify_github_signature_invalid() {
    let secret = "test_secret";
    let body = r#"{"action": "opened", "issue": {"node_id": "test_node_id"}}"#;

    let mut headers = HeaderMap::new();
    headers.insert("X-Hub-Signature-256", "invalid_signature".parse().unwrap());

    assert!(!verify_github_signature(secret, &headers, body));
}

#[tokio::test]
async fn test_handle_webhook_issue_opened() {
    let state = create_mock_state();
    let body = r#"{"action": "opened", "issue": {"node_id": "test_node_id"}}"#;

    let mut mac = Hmac::<Sha256>::new_from_slice(state.webhook_secret.as_bytes()).unwrap();
    mac.update(body.as_bytes());
    let result = mac.finalize();
    let signature = format!("sha256={}", hex::encode(result.into_bytes()));

    let mut headers = HeaderMap::new();
    headers.insert("X-Hub-Signature-256", signature.parse().unwrap());

    let result = handle_webhook(State(state), headers, body.to_string()).await;
    assert_eq!(result, Ok(StatusCode::OK));
}

#[tokio::test]
async fn test_handle_webhook_pr_opened() {
    let state = create_mock_state();
    let body = r#"{"action": "opened", "pull_request": {"node_id": "test_node_id"}}"#;

    let mut mac = Hmac::<Sha256>::new_from_slice(state.webhook_secret.as_bytes()).unwrap();
    mac.update(body.as_bytes());
    let result = mac.finalize();
    let signature = format!("sha256={}", hex::encode(result.into_bytes()));

    let mut headers = HeaderMap::new();
    headers.insert("X-Hub-Signature-256", signature.parse().unwrap());

    let result = handle_webhook(State(state), headers, body.to_string()).await;
    assert_eq!(result, Ok(StatusCode::OK));
}

#[tokio::test]
async fn test_handle_webhook_invalid_signature() {
    let state = create_mock_state();
    let body = r#"{"action": "opened", "issue": {"node_id": "test_node_id"}}"#;

    let mut headers = HeaderMap::new();
    headers.insert("X-Hub-Signature-256", "invalid_signature".parse().unwrap());

    let result = handle_webhook(State(state), headers, body.to_string()).await;
    assert_eq!(result, Err(StatusCode::UNAUTHORIZED));
}

#[tokio::test]
async fn test_handle_webhook_unsupported_action() {
    let state = create_mock_state();
    let body = r#"{"action": "closed", "issue": {"node_id": "test_node_id"}}"#;

    let mut mac = Hmac::<Sha256>::new_from_slice(state.webhook_secret.as_bytes()).unwrap();
    mac.update(body.as_bytes());
    let result = mac.finalize();
    let signature = format!("sha256={}", hex::encode(result.into_bytes()));

    let mut headers = HeaderMap::new();
    headers.insert("X-Hub-Signature-256", signature.parse().unwrap());

    let result = handle_webhook(State(state), headers, body.to_string()).await;
    assert_eq!(result, Ok(StatusCode::OK));
}

// Note: This test requires mocking the Octocrab client, which is beyond the scope of this example.
// In a real-world scenario, you'd want to use a mocking library like mockall to create a mock Octocrab client.
// #[tokio::test]
// async fn test_add_to_project() {
//     // Implement this test with a mocked Octocrab client
// }
