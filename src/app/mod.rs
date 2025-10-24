use anyhow::Result;
use axum::{serve, Router};
use std::net::SocketAddr;
use tokio::net::TcpListener;

use crate::{
    db::Repo,
    http::build_router,
    templates::TemplateService,
    tenancy::TenantResolver,
};

#[derive(Clone)]
pub struct AppState {
    pub tenants: TenantResolver,
    pub tmpl: TemplateService,
    pub repo: Repo,
}

pub async fn run() -> Result<()> {
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let repo = Repo::new(&db_url).await?;
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
