use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct PullRequest {
    pub node_id: String,
}
