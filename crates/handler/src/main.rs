use anyhow::Error;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Router,
};
use azure_security_keyvault::KeyvaultClient;
use hmac::{Hmac, Mac};
use octocrab::Octocrab;
use serde::Deserialize;
use sha2::Sha256;
use std::{env, sync::Arc};

#[cfg(test)]
#[path = "main_tests.rs"]
mod main_tests;

struct AppState {
    octocrab: Octocrab,
    project_id: String,
    webhook_secret: String,
}

#[derive(Deserialize)]
struct WebhookPayload {
    action: String,
    issue: Option<Issue>,
    pull_request: Option<PullRequest>,
}

#[derive(Deserialize)]
struct Issue {
    node_id: String,
}

#[derive(Deserialize)]
struct PullRequest {
    node_id: String,
}

async fn add_to_project(
    octocrab: &Octocrab,
    project_id: &str,
    item_id: String,
) -> Result<(), StatusCode> {
    let query = format!(
        r#"
        mutation {{
            addProjectV2ItemById(input: {{projectId: "{}", contentId: "{}"}}) {{
                item {{
                    id
                }}
            }}
        }}
        "#,
        project_id, item_id
    );

    octocrab
        .graphql(&query)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(())
}

async fn get_secret_from_keyvault(key_vault_url: &str, secret_name: &str) -> Result<String, Error> {
    let credential = azure_identity::create_credential()?;
    let client = KeyvaultClient::new(&key_vault_url, credential)?;

    let secret = client.secret_client().get(secret_name).await?;
    Ok(secret.value)
}

async fn handle_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> Result<StatusCode, StatusCode> {
    // Verify GitHub signature
    if !verify_github_signature(&state.webhook_secret, &headers, &body) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let payload: WebhookPayload =
        serde_json::from_str(&body).map_err(|_| StatusCode::BAD_REQUEST)?;

    match payload.action.as_str() {
        "opened" => {
            if let Some(issue) = payload.issue {
                add_to_project(&state.octocrab, &state.project_id, issue.node_id).await?;
            } else if let Some(pr) = payload.pull_request {
                add_to_project(&state.octocrab, &state.project_id, pr.node_id).await?;
            }
        }
        _ => return Ok(StatusCode::OK),
    }

    Ok(StatusCode::OK)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let key_vault_name = env::var("KEY_VAULT_NAME")?;
    let key_vault_url = format!("https://{}.vault.azure.net", key_vault_name);

    let github_token = get_secret_from_keyvault(key_vault_url.as_str(), "GITHUB-TOKEN").await?;
    let project_id = get_secret_from_keyvault(key_vault_url.as_str(), "GITHUB-PROJECT-ID").await?;
    let webhook_secret =
        get_secret_from_keyvault(key_vault_url.as_str(), "GITHUB-WEBHOOK-SECRET").await?;

    let octocrab = Octocrab::builder().personal_token(github_token).build()?;

    let state = Arc::new(AppState {
        octocrab,
        project_id,
        webhook_secret,
    });

    let app = Router::new()
        .route("/webhook", post(handle_webhook))
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    // println!("Listening on {}", addr);
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

fn verify_github_signature(secret: &str, headers: &HeaderMap, body: &str) -> bool {
    let signature = match headers.get("X-Hub-Signature-256") {
        Some(value) => value.to_str().unwrap_or(""),
        None => return false,
    };

    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(body.as_bytes());
    let result = mac.finalize();
    let computed_signature = format!("sha256={}", hex::encode(result.into_bytes()));

    signature == computed_signature
}
