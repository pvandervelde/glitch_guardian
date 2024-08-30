use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct Installation {
    pub id: u64,
    pub node_id: String,
}
