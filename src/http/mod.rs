use crate::{app::AppState, data::ContextBuilder};
use axum::{
    Router,
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header::CONTENT_TYPE},
    response::{Html, Response},
    routing::get,
};
use minijinja::ErrorKind as TemplateErrorKind;
use serde::Deserialize;
use serde_json::json;
use std::path::{Component, Path as StdPath, PathBuf};
use tokio::fs;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/static/*path", get(serve_static))
        .route("/favicon.ico", get(serve_favicon))
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

#[derive(Default, Deserialize)]
struct QueryParams {
    #[serde(flatten)]
    params: serde_json::Map<String, serde_json::Value>,
}

async fn render_dynamic(
    headers: HeaderMap,
    Query(template_override): Query<TemplateOverride>,
    Query(query_params): Query<QueryParams>,
    Path(TenantPath { tenant, path }): Path<TenantPath>,
    State(state): State<AppState>,
) -> Result<Html<String>, (StatusCode, String)> {
    let clean_path = path.unwrap_or_else(|| "/".to_string());
    let db_path = if clean_path.starts_with('/') {
        clean_path.clone()
    } else {
        format!("/{}", clean_path)
    };
    let normalized_path = clean_path.trim_start_matches('/');
    let mut params_map = query_params.params.clone();
    let product_slug = normalized_path
        .strip_prefix("products/")
        .filter(|slug| !slug.is_empty())
        .map(|slug| slug.to_string());
    if let Some(slug) = &product_slug {
        params_map.insert("slug".to_string(), json!(slug));
        params_map.insert("product_slug".to_string(), json!(slug));
        params_map.insert("product_id".to_string(), json!(slug));
    }

    let tenant = state
        .tenants
        .resolve(&headers, &tenant)
        .await
        .map_err(internal)?;
    let mut route = state
        .repo
        .find_route(&tenant, &db_path)
        .await
        .map_err(internal)?;
    if route.is_none() {
        if normalized_path == "product" {
            route = state
                .repo
                .find_route(&tenant, "/product")
                .await
                .map_err(internal)?;
        } else if product_slug.is_some() {
            route = state
                .repo
                .find_route(&tenant, "/product")
                .await
                .map_err(internal)?;
        }
    }

    let template_name = template_override
        .template
        .clone()
        .or_else(|| route.as_ref().map(|r| r.template_name.clone()))
        .or_else(|| {
            if product_slug.is_some() {
                Some("pages/product.html".to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| infer_template_name(&clean_path));

    let data_source = route
        .as_ref()
        .map(|r| r.data_source.clone())
        .unwrap_or_else(|| {
            if product_slug.is_some() {
                let product_source = json!({
                    "provider": "http",
                    "url": "https://api.mobicms.com.br/api/furnitures/{{product_id}}",
                    "method": "GET",
                    "headers": {
                        "Authorization": "Bearer {{env.MOBI_API_TOKEN}}"
                    }
                });
                json!({
                    "provider": "static",
                    "payload": {
                        "product": product_source
                    }
                })
            } else {
                json!({ "provider": "static", "payload": {} })
            }
        });

    let env = state.tmpl.env_for(&tenant).await.map_err(internal)?;
    let ctx = ContextBuilder::from_source(&state.repo, &tenant, &data_source, &params_map)
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

async fn serve_static(Path(path): Path<String>) -> Result<Response, (StatusCode, String)> {
    let clean_path = sanitize_path(&path).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "invalid path segment requested".to_string(),
        )
    })?;

    let base = PathBuf::from("static");
    let full_path = base.join(clean_path);

    let data = fs::read(&full_path).await.map_err(|err| match err.kind() {
        std::io::ErrorKind::NotFound => (
            StatusCode::NOT_FOUND,
            format!("static asset not found: {}", full_path.display()),
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to read static asset: {err}"),
        ),
    })?;

    let mime = mime_for(&full_path);
    let mut response = Response::new(Body::from(data));
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_str(mime)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );

    Ok(response)
}

async fn serve_favicon() -> Result<Response, (StatusCode, String)> {
    let base = PathBuf::from("static");
    let full_path = base.join("favicon.ico");

    let data = fs::read(&full_path).await.map_err(|err| match err.kind() {
        std::io::ErrorKind::NotFound => (
            StatusCode::NOT_FOUND,
            format!("favicon not found: {}", full_path.display()),
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to read favicon: {err}"),
        ),
    })?;

    let mut response = Response::new(Body::from(data));
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("image/x-icon"));

    Ok(response)
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

fn sanitize_path(path: &str) -> Option<PathBuf> {
    let mut clean = PathBuf::new();
    for component in StdPath::new(path).components() {
        match component {
            Component::Normal(part) => clean.push(part),
            Component::CurDir => {}
            _ => return None,
        }
    }
    Some(clean)
}

fn mime_for(path: &StdPath) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_ascii_lowercase())
    {
        Some(ext) => match ext.as_str() {
            "js" | "mjs" => "application/javascript",
            "json" => "application/json",
            "css" => "text/css",
            "html" => "text/html; charset=utf-8",
            "wasm" => "application/wasm",
            "svg" => "image/svg+xml",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "webp" => "image/webp",
            "ico" => "image/x-icon",
            _ => "application/octet-stream",
        },
        None => "application/octet-stream",
    }
}
