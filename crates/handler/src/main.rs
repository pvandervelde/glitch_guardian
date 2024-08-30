mod api;
mod telemetry;

#[cfg(test)]
#[path = "main_tests.rs"]
mod main_tests;

use anyhow::{anyhow, Error};
use api::{
    installations::Installation, issues::Issue, projects::add_to_project, prs::PullRequest,
    repositories::Repository,
};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Router,
};
use azure_security_keyvault::KeyvaultClient;
use dotenv::dotenv;
use hmac::{Hmac, Mac};
use jsonwebtoken::EncodingKey;
use octocrab::{
    models::{InstallationId, InstallationToken},
    params::apps::CreateInstallationAccessToken,
    Octocrab, Page,
};
use serde::Deserialize;
use sha2::Sha256;
use std::{env, sync::Arc};
use tracing::{debug, info, instrument, warn};

struct AppConfig {
    app_id: u64,
    app_private_key: String,
    project_id: String,
    webhook_secret: String,
    port_number: u16,
}

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
    repository: Option<Repository>,
    installation: Option<Installation>,
}

async fn authenticate_with_access_token(
    octocrab: &Octocrab,
    installation_id: u64,
    source_repository: &str,
) -> Result<Octocrab, Error> {
    // Get an access token for the specific app installation that sent the event
    // First find all the installations and use those to grab the specific one that
    // sent the event
    let installations = octocrab
        .apps()
        .installations()
        .send()
        .await
        .unwrap_or(Page::<octocrab::models::Installation>::default())
        .take_items();

    let repository_name = source_repository.to_string();
    let id = InstallationId(installation_id);
    let Some(installation_index) = installations.iter().position(|l| l.id == id) else {
        return Err(anyhow!(
            "Failed to find installation with id {}",
            installation_id
        ));
    };

    let installation = &installations[installation_index];
    debug!(
        "Creating access token for installation with id {}. Linked to repository at {}",
        installation.id,
        installation
            .repositories_url
            .clone()
            .unwrap_or("".to_string())
    );

    let mut create_access_token = CreateInstallationAccessToken::default();
    //create_access_token.repositories = vec![repository_name.clone()];

    // Create an access token for the installation
    let access: InstallationToken = octocrab
        .post(
            installations[installation_index]
                .access_tokens_url
                .as_ref()
                .unwrap(),
            Some(&create_access_token),
        )
        .await?;

    // USe the API token
    let api_with_token = octocrab::OctocrabBuilder::new()
        .personal_token(access.token)
        .build()
        .unwrap();

    Ok(api_with_token)
}

async fn create_github_app_client(app_id: u64, private_key: &str) -> Result<Octocrab, Error> {
    //let app_id_struct = AppId::from(app_id);
    let key = EncodingKey::from_rsa_pem(private_key.as_bytes())?;

    //let octocrab = Octocrab::builder().app(app_id_struct, key).build()?;

    let token = octocrab::auth::create_jwt(app_id.into(), &key).unwrap();
    let octocrab = Octocrab::builder().personal_token(token).build()?;

    Ok(octocrab)
}

async fn get_azure_config() -> Result<AppConfig, Error> {
    let key_vault_name = env::var("KEY_VAULT_NAME")?;
    let key_vault_url = format!("https://{}.vault.azure.net", key_vault_name);

    debug!(
        "Fetching configuration from Azure Key Vault at: {}",
        key_vault_url.as_str()
    );
    let app_id = get_secret_from_keyvault(key_vault_url.as_str(), "GithubAppId").await?;
    let app_private_key =
        get_secret_from_keyvault(key_vault_url.as_str(), "GithubAppPrivateKey").await?;

    let project_id = get_secret_from_keyvault(key_vault_url.as_str(), "GithubProjectId").await?;
    let webhook_secret =
        get_secret_from_keyvault(key_vault_url.as_str(), "GithubWebhookSecret").await?;

    let port_key = "FUNCTIONS_CUSTOMHANDLER_PORT";
    let port: u16 = match env::var(port_key) {
        Ok(val) => val.parse().expect("Custom Handler port is not a number!"),
        Err(_) => 3000,
    };

    let app_id_to_number = app_id.parse::<u64>()?;
    let config = AppConfig {
        app_id: app_id_to_number,
        app_private_key,
        project_id,
        webhook_secret,
        port_number: port,
    };

    Ok(config)
}

fn get_local_config() -> Result<AppConfig, Error> {
    dotenv().ok();

    let app_id = env::var("GITHUB_APP_ID")?;
    let app_private_key = env::var("GITHUB_APP_PRIVATE_KEY")?;
    let project_id = env::var("GITHUB_PROJECT_ID")?;
    let webhook_secret = env::var("GITHUB_WEBHOOK_SECRET")?;

    let app_id_to_number = app_id.parse::<u64>()?;
    let config = AppConfig {
        app_id: app_id_to_number,
        app_private_key,
        project_id,
        webhook_secret,
        port_number: 3000,
    };

    Ok(config)
}

async fn get_secret_from_keyvault(key_vault_url: &str, secret_name: &str) -> Result<String, Error> {
    let credential = azure_identity::create_credential()?;
    let client = KeyvaultClient::new(key_vault_url, credential)?;

    let secret = client.secret_client().get(secret_name).await?;
    Ok(secret.value)
}

#[instrument(skip(state, headers, body))]
async fn handle_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> Result<StatusCode, StatusCode> {
    info!("Received webhook call from Github");

    if !verify_github_signature(&state.webhook_secret, &headers, &body) {
        warn!("Webhook did not have valid signature");
        return Err(StatusCode::UNAUTHORIZED);
    }

    info!("Webhook has valid signature. Processing information ...");

    let payload: WebhookPayload =
        serde_json::from_str(&body).map_err(|_| StatusCode::BAD_REQUEST)?;

    let Some(installation) = payload.installation else {
        return Err(StatusCode::BAD_REQUEST);
    };
    let installation_id = installation.id;

    let Some(repository) = payload.repository else {
        return Err(StatusCode::BAD_REQUEST);
    };

    let api_with_pat =
        match authenticate_with_access_token(&state.octocrab, installation_id, &repository.name)
            .await
        {
            Ok(o) => o,
            Err(_) => return Err(StatusCode::UNAUTHORIZED),
        };

    debug!("Github action is {}", payload.action.as_str());
    match payload.action.as_str() {
        "opened" => {
            let response = if let Some(issue) = payload.issue {
                debug!("Issue created with ID: {}", issue.node_id);
                add_to_project(&api_with_pat, &state.project_id, issue.node_id).await
            } else if let Some(pr) = payload.pull_request {
                debug!("PR created with ID: {}", pr.node_id);
                add_to_project(&api_with_pat, &state.project_id, pr.node_id).await
            } else {
                Ok(())
            };

            if response.is_err() {
                let err = response.err().unwrap();
                warn!("Request to Github failed. Response was: {}", &err);

                return Err(err);
            }
        }
        _ => return Ok(StatusCode::OK),
    }

    Ok(StatusCode::OK)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let is_azure = env::var("AZURE_FUNCTIONS_ENVIRONMENT").is_ok();

    // Initialize telemetry
    if is_azure {
        telemetry::init_azure_telemetry().await?;
    } else {
        telemetry::init_local_telemetry()?;
    }

    info!("Starting application");

    let config_values = if is_azure {
        debug!("Running in Azure. Loading Azure configs ...");
        get_azure_config().await?
    } else {
        debug!("Running in local mode. Loading local configs ...");
        get_local_config()?
    };

    let octocrab =
        create_github_app_client(config_values.app_id, &config_values.app_private_key).await?;

    let state = Arc::new(AppState {
        octocrab,
        project_id: config_values.project_id,
        webhook_secret: config_values.webhook_secret,
    });

    let app = Router::new()
        .route("/webhook", post(handle_webhook))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", config_values.port_number);
    let listener = tokio::net::TcpListener::bind(addr.clone()).await.unwrap();

    info!("Listening on {}", addr);
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

#[instrument]
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
