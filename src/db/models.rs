use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub template_name: String,
    pub data_source: Json,
}
