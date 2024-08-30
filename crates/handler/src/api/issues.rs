use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct Issue {
    pub node_id: String,
}
