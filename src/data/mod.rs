use crate::db::Repo;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Value as Json, json};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum DataSourceCfg {
    Static { payload: Json },
    DbQuery { sql: String, params: Option<Json> },
    Http { url: String, method: Option<String> },
}

pub struct ContextBuilder;

impl ContextBuilder {
    pub async fn from_source(repo: &Repo, tenant: &str, source: &Json) -> Result<minijinja::Value> {
        let mut v = match serde_json::from_value::<DataSourceCfg>(source.clone())? {
            DataSourceCfg::Static { payload } => payload,
            DataSourceCfg::DbQuery { sql, params } => repo.json_query(tenant, &sql, params).await?,
            DataSourceCfg::Http { url, method } => {
                serde_json::json!({ "_todo": {"url": url, "method": method.unwrap_or("GET".into()) } })
            }
        };

        v["site"] = json!({
            "title": tenant,
            "slug": tenant,
        });

        Ok(minijinja::Value::from_serialize(&v))
    }
}
