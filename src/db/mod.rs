use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value as Json, json};
use std::{collections::HashMap, fs, path::Path, path::PathBuf, sync::Arc};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RouteCfg {
    path: String,
    template_name: String,
    #[serde(default)]
    data_source: Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    #[serde(default)]
    tenants: Vec<String>,
    #[serde(default)]
    routes: HashMap<String, Vec<RouteCfg>>, // tenant_slug -> routes
}

#[derive(Clone)]
pub struct Repo {
    config: Arc<Config>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub template_name: String,
    pub data_source: Json,
}

impl Repo {
    pub async fn new(config_path: &str) -> Result<Self> {
        let path = PathBuf::from(config_path);
        let cfg = load_config(&path)?;
        Ok(Self {
            config: Arc::new(cfg),
        })
    }

    pub async fn find_route(&self, tenant: &str, path: &str) -> Result<Option<Route>> {
        let routes = self
            .config
            .routes
            .get(tenant)
            .or_else(|| self.config.routes.get("_shared"));
        if let Some(list) = routes {
            if let Some(rc) = list.iter().find(|r| r.path == path) {
                return Ok(Some(Route {
                    template_name: rc.template_name.clone(),
                    data_source: rc.data_source.clone(),
                }));
            }
        }
        Ok(None)
    }

    pub async fn json_query(
        &self,
        _tenant: &str,
        _sql: &str,
        _params: Option<Json>,
    ) -> Result<Json> {
        // No SQL backend in file mode. Return empty array for now.
        Ok(json!([]))
    }

    pub async fn tenant_exists(&self, slug: &str) -> Result<bool> {
        let in_list = self.config.tenants.iter().any(|s| s == slug);
        let in_routes = self.config.routes.contains_key(slug);
        Ok(in_list || in_routes)
    }
}

fn load_config(path: &Path) -> Result<Config> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("reading routes config from {}", path.display()))?;
    let cfg: Config = serde_json::from_str(&text)
        .with_context(|| format!("parsing routes config from {}", path.display()))?;
    Ok(cfg)
}
