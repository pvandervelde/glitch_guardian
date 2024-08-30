use octocrab::Octocrab;
use reqwest::StatusCode;
use tracing::instrument;

#[instrument(skip(octocrab))]
pub(crate) async fn add_to_project(
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
