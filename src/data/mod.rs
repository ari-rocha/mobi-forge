use crate::db::Repo;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value as Json, json};
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum DataSourceCfg {
    Static {
        payload: Json,
    },
    DbQuery {
        sql: String,
        params: Option<Json>,
    },
    Http {
        url: String,
        method: Option<String>,
        headers: Option<Json>,
    },
    MockFile {
        path: String,
    },
}

pub struct ContextBuilder;

impl ContextBuilder {
    pub async fn from_source(
        repo: &Repo,
        tenant: &str,
        source: &Json,
        query_params: &serde_json::Map<String, Json>,
    ) -> Result<minijinja::Value> {
        let mut v = Self::process_source(repo, tenant, source, query_params).await?;

        if let Some(site) = v.get_mut("site").and_then(|value| value.as_object_mut()) {
            site.entry("slug".to_string())
                .or_insert_with(|| json!(tenant));
            site.entry("title".to_string())
                .or_insert_with(|| json!(tenant));
        } else {
            v["site"] = json!({
                "title": tenant,
                "slug": tenant,
            });
        }

        // Process nested data sources in the response (e.g., results, products)
        if let Some(obj) = v.as_object_mut() {
            let keys_to_process: Vec<String> = obj
                .iter()
                .filter_map(|(key, val)| {
                    if val.is_object() && val.get("provider").is_some() {
                        Some(key.clone())
                    } else {
                        None
                    }
                })
                .collect();

            for key in keys_to_process {
                if let Some(nested_source) = obj.get(&key).cloned() {
                    if let Ok(mut nested_value) =
                        Self::process_source(repo, tenant, &nested_source, query_params).await
                    {
                        if let Some(data_obj) = nested_value.as_object_mut() {
                            if let Some(data_value) = data_obj.remove("data") {
                                nested_value = data_value;
                            }
                        }
                        obj.insert(key, nested_value);
                    }
                }
            }
        }

        if let Some(obj) = v.as_object_mut() {
            for (key, value) in query_params.iter() {
                obj.entry(key.clone()).or_insert(value.clone());
            }

            if let Some(q_value) = query_params.get("q") {
                if let Some(page) = obj.get_mut("page").and_then(|p| p.as_object_mut()) {
                    page.insert("query".to_string(), q_value.clone());
                }
            }
        }

        Ok(minijinja::Value::from_serialize(&v))
    }

    async fn process_source(
        repo: &Repo,
        tenant: &str,
        source: &Json,
        query_params: &serde_json::Map<String, Json>,
    ) -> Result<Json> {
        let data_source_cfg =
            if let Ok(cfg) = serde_json::from_value::<DataSourceCfg>(source.clone()) {
                cfg
            } else {
                DataSourceCfg::Static {
                    payload: source.clone(),
                }
            };

        match data_source_cfg {
            DataSourceCfg::Static { payload } => Ok(payload),
            DataSourceCfg::DbQuery { sql, params } => repo.json_query(tenant, &sql, params).await,
            DataSourceCfg::Http {
                url,
                method,
                headers,
            } => {
                let final_url = render_placeholder_string(&url, query_params);

                let client = reqwest::Client::new();
                let method = method.unwrap_or_else(|| "GET".to_string()).to_uppercase();

                let mut req = match method.as_str() {
                    "POST" => client.post(&final_url),
                    "PUT" => client.put(&final_url),
                    "PATCH" => client.patch(&final_url),
                    "DELETE" => client.delete(&final_url),
                    _ => client.get(&final_url),
                };

                if let Some(headers_obj) = headers {
                    if let Some(headers_map) = headers_obj.as_object() {
                        for (key, value) in headers_map {
                            if let Some(val_str) = value.as_str() {
                                let rendered = render_placeholder_string(val_str, query_params);
                                req = req.header(key.clone(), rendered);
                            }
                        }
                    }
                }

                let response = req
                    .send()
                    .await
                    .with_context(|| format!("failed to fetch from {}", final_url))?;

                let status = response.status();

                if !status.is_success() {
                    let error_body = response.text().await.unwrap_or_default();
                    anyhow::bail!("HTTP error {}: {}", status, error_body);
                }

                let body = response
                    .text()
                    .await
                    .with_context(|| "failed to read response")?;

                let parsed =
                    serde_json::from_str::<Json>(&body).with_context(|| "failed to parse JSON")?;

                Ok(parsed)
            }
            DataSourceCfg::MockFile { path } => {
                let base = std::env::var("MOCK_DATA_DIR").unwrap_or_else(|_| "mock-data".into());
                let resolved = PathBuf::from(base).join(path);
                let raw = fs::read_to_string(&resolved)
                    .await
                    .with_context(|| format!("reading mock data file {:?}", resolved))?;
                serde_json::from_str(&raw)
                    .with_context(|| format!("parsing JSON from {:?}", resolved))
            }
        }
    }
}

fn render_placeholder_string(
    template: &str,
    query_params: &serde_json::Map<String, Json>,
) -> String {
    let mut out = template.to_string();
    for (key, value) in query_params.iter() {
        if let Some(val_str) = value.as_str() {
            let placeholder = format!("{{{{{}}}}}", key);
            out = out.replace(&placeholder, val_str);
        }
    }

    let mut rendered = String::new();
    let mut rest = out.as_str();

    while let Some(start) = rest.find("{{env.") {
        rendered.push_str(&rest[..start]);
        let after = &rest[start + 6..];
        if let Some(end) = after.find("}}") {
            let key = &after[..end];
            let value = std::env::var(key).unwrap_or_default();
            rendered.push_str(&value);
            rest = &after[end + 2..];
        } else {
            rendered.push_str(&rest[start..]);
            rest = "";
            break;
        }
    }

    rendered.push_str(rest);
    rendered
}
