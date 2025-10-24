use anyhow::{Result, anyhow};
use axum::http::HeaderMap;

use crate::db::Repo;

#[derive(Clone)]
pub struct TenantResolver {
    pub repo: Repo,
}

impl TenantResolver {
    pub fn new(repo: Repo) -> Self {
        Self { repo }
    }

    pub async fn resolve(&self, _headers: &HeaderMap, tenant_slug: &str) -> Result<String> {
        let exists = self.repo.tenant_exists(tenant_slug).await?;
        if exists {
            Ok(tenant_slug.to_string())
        } else {
            Err(anyhow!("tenant not found"))
        }
    }
}
