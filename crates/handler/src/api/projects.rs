use graphql_client::GraphQLQuery;
use octocrab::{
    models::{InstallationId, InstallationToken},
    params::apps::CreateInstallationAccessToken,
    Octocrab, Page,
};
use reqwest::StatusCode;
use tracing::{debug, info, instrument, warn};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/graphql/github_schema.graphql",
    query_path = "schemas/graphql/mutation_gh_project_add_item.graphql",
    variables_derives = "Clone, Debug",
    response_derives = "Clone, Debug"
)]
struct AddItemToProjectMutation;

#[instrument(skip(octocrab))]
pub(crate) async fn add_to_project(
    octocrab: &Octocrab,
    project_id: &str,
    item_id: String,
) -> Result<(), StatusCode> {
    let variables = add_item_to_project_mutation::Variables {
        project_id: project_id.to_string(),
        content_id: item_id.clone(),
    };

    let payload = &AddItemToProjectMutation::build_query(variables.clone());
    let _json = &serde_json::json!(payload);

    info!("Updating project [{}] with item [{}]", project_id, item_id);
    debug!("Sending payload: [{}]", _json);
    let response: octocrab::Result<
        graphql_client::Response<add_item_to_project_mutation::ResponseData>,
    > = octocrab.graphql(payload).await;

    debug!("processing response...");
    if response.is_ok() {
        let resp = response.unwrap();
        let resp_to_string = match resp.data {
            Some(d) => match d.add_project_v2_item_by_id {
                Some(a) => match a.item {
                    Some(i) => i.id,
                    None => "".to_string(),
                },
                None => "".to_string(),
            },
            None => "".to_string(),
        };

        let errors_as_string = match resp.errors {
            Some(e) => e.iter().map(|x| x.to_string() + ",").collect::<String>(),
            None => "".to_string(),
        };

        info!("Response: {}. Errors: {}", resp_to_string, errors_as_string);

        return Ok(());
    } else {
        warn!("Response type indicates failure ...");
        let err = response.err().unwrap();
        match err {
            octocrab::Error::GitHub { source, .. } => {
                let errors = source
                    .errors
                    .unwrap_or_default()
                    .iter()
                    .map(|x| x.to_string() + ",")
                    .collect::<String>();
                warn!(
                    error_message = source.message,
                    errors = errors,
                    status_code = source.status_code.to_string(),
                    error_type = "github",
                    "Updating project failed"
                )
            }
            octocrab::Error::UriParse { source, .. } => {
                warn!(
                    error_message = source.to_string(),
                    error_type = "uriparse",
                    "Updating project failed"
                )
            }
            octocrab::Error::Uri { source, .. } => {
                warn!(
                    error_message = source.to_string(),
                    error_type = "uri",
                    "Updating project failed"
                )
            }
            octocrab::Error::InvalidHeaderValue { source, .. } => {
                warn!(
                    error_message = source.to_string(),
                    error_type = "invalidheadervalue",
                    "Updating project failed"
                )
            }
            octocrab::Error::Http { source, .. } => {
                warn!(
                    error_message = source.to_string(),
                    error_type = "http",
                    "Updating project failed"
                )
            }
            octocrab::Error::InvalidUtf8 { source, .. } => {
                warn!(
                    error_message = source.to_string(),
                    error_type = "invalidutf8",
                    "Updating project failed"
                )
            }
            octocrab::Error::Encoder { source, .. } => {
                warn!(
                    error_message = source.to_string(),
                    error_type = "encoder",
                    "Updating project failed"
                )
            }
            octocrab::Error::Service { source, .. } => {
                warn!(
                    error_message = source.to_string(),
                    error_type = "service",
                    "Updating project failed"
                )
            }
            octocrab::Error::Hyper { source, .. } => warn!(
                error_message = source.to_string(),
                error_type = "hyper",
                "Updating project failed"
            ),
            octocrab::Error::SerdeUrlEncoded { source, .. } => warn!(
                error_message = source.to_string(),
                error_type = "serdeurlencoded",
                "Updating project failed"
            ),
            octocrab::Error::Serde { source, .. } => warn!(
                error_message = source.to_string(),
                error_type = "serde",
                "Updating project failed"
            ),
            octocrab::Error::Json { source, .. } => warn!(
                path = source.path().to_string(),
                error_message = source.to_string(),
                error_type = "json",
                "Updating project failed"
            ),
            octocrab::Error::JWT { source, .. } => warn!(
                error_message = source.to_string(),
                error_type = "jwt",
                "Updating project failed"
            ),
            octocrab::Error::Other { source, .. } => warn!(
                error_message = source.to_string(),
                error_type = "other",
                "Updating project failed"
            ),
        };

        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
}
