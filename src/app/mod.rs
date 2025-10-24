use anyhow::Result;
use axum::{Router, serve};
use std::net::SocketAddr;
use tokio::net::TcpListener;

use crate::{db::Repo, http::build_router, templates::TemplateService, tenancy::TenantResolver};

#[derive(Clone)]
pub struct AppState {
    pub tenants: TenantResolver,
    pub tmpl: TemplateService,
    pub repo: Repo,
}

pub async fn run() -> Result<()> {
    let routes_file = std::env::var("ROUTES_FILE").unwrap_or_else(|_| "config/routes.json".into());
    let repo = Repo::new(&routes_file).await?;
    let template_dir = std::env::var("TEMPLATE_DIR").unwrap_or_else(|_| "templates".into());

    let state = AppState {
        tenants: TenantResolver::new(repo.clone()),
        tmpl: TemplateService::new(template_dir),
        repo: repo.clone(),
    };

    let app: Router = build_router(state);

    let addr: SocketAddr = "0.0.0.0:8080".parse()?;
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("listening on http://{}", addr);

    serve(listener, app).await?;
    Ok(())
}
