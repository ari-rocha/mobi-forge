pub mod models;
pub use models::Route;

use anyhow::Result;
use serde_json::Value as Json;
use sqlx::{Column, PgPool, Row};

#[derive(Clone)]
pub struct Repo {
    pub pool: PgPool,
}

impl Repo {
    pub async fn new(url: &str) -> Result<Self> {
        Ok(Self {
            pool: PgPool::connect(url).await?,
        })
    }

    pub async fn find_route(&self, tenant: &str, path: &str) -> Result<Option<Route>> {
        let rec = sqlx::query_as::<_, Route>(
            r#"
SELECT r.id, r.template_name, r.data_source, r.updated_at
FROM routes r
JOIN tenants t ON t.id = r.tenant_id
WHERE t.slug = $1 AND r.path = $2 AND r.is_published = true
"#,
        )
        .bind(tenant)
        .bind(path)
        .fetch_optional(&self.pool)
        .await?;
        Ok(rec)
    }

    pub async fn json_query(
        &self,
        _tenant: &str,
        sql: &str,
        _params: Option<Json>,
    ) -> Result<Json> {
        let rows = sqlx::query(sql).fetch_all(&self.pool).await?;
        let arr: Vec<Json> = rows
            .into_iter()
            .map(|r| {
                let mut map = serde_json::Map::new();
                for (i, col) in r.columns().iter().enumerate() {
                    map.insert(col.name().to_string(), json_from_sqlx(&r, i));
                }
                Json::Object(map)
            })
            .collect();
        Ok(Json::Array(arr))
    }

    pub async fn tenant_exists(&self, slug: &str) -> Result<bool> {
        let exists: Option<bool> = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT TRUE
            FROM tenants
            WHERE slug = $1
            LIMIT 1
            "#,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;

        Ok(exists.unwrap_or(false))
    }
}

fn json_from_sqlx(row: &sqlx::postgres::PgRow, idx: usize) -> Json {
    row.try_get::<String, _>(idx)
        .map(Json::String)
        .unwrap_or(Json::Null)
}
