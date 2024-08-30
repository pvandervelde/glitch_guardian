use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct Repository {
    pub full_name: String,
    pub name: String,
    pub node_id: String,
    pub private: bool,
}
