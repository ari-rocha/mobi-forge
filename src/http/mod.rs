use crate::{app::AppState, data::ContextBuilder};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Html,
    routing::get,
    Router,
};
use minijinja::ErrorKind as TemplateErrorKind;
use serde::Deserialize;
use serde_json::json;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/@:tenant", get(render_dynamic))
        .route("/@:tenant/", get(render_dynamic))
        .route("/@:tenant/*path", get(render_dynamic))
        .route("/*path", get(render_dynamic))
        .with_state(state)
}

#[derive(Deserialize)]
struct TenantPath {
    tenant: String,
    path: Option<String>,
}

#[derive(Default, Deserialize)]
struct TemplateOverride {
    #[serde(default)]
    template: Option<String>,
}

async fn render_dynamic(
    headers: HeaderMap,
    Query(template_override): Query<TemplateOverride>,
    Path(TenantPath { tenant, path }): Path<TenantPath>,
    State(state): State<AppState>,
) -> Result<Html<String>, (StatusCode, String)> {
    let clean_path = path.unwrap_or_else(|| "/".to_string());

    let tenant = state
        .tenants
        .resolve(&headers, &tenant)
        .await
        .map_err(internal)?;
    let route = state
        .repo
        .find_route(&tenant, &clean_path)
        .await
        .map_err(internal)?;

    let template_name = template_override
        .template
        .clone()
        .or_else(|| route.as_ref().map(|r| r.template_name.clone()))
        .unwrap_or_else(|| infer_template_name(&clean_path));

    let data_source = route
        .as_ref()
        .map(|r| r.data_source.clone())
        .unwrap_or_else(|| json!({ "provider": "static", "payload": {} }));

    let env = state.tmpl.env_for(&tenant).await.map_err(internal)?;
    let ctx = ContextBuilder::from_source(&state.repo, &tenant, &data_source)
        .await
        .map_err(internal)?;

    let tpl = env
        .get_template(&template_name)
        .map_err(|err| match err.kind() {
            TemplateErrorKind::TemplateNotFound => (StatusCode::NOT_FOUND, err.to_string()),
            _ => internal(err),
        })?;
    let html = tpl.render(ctx).map_err(internal)?;
    Ok(Html(html))
}

fn internal<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

fn infer_template_name(path: &str) -> String {
    let normalized = path.trim().trim_start_matches('/');
    if normalized.is_empty() || normalized.ends_with('/') {
        "index.html".to_string()
    } else if normalized.contains('.') {
        normalized.to_string()
    } else {
        format!("{normalized}.html")
    }
}
