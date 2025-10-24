use anyhow::{Context, Result};
use chrono::Utc;
use minijinja::{AutoEscape, Environment, Error, ErrorKind, value::Value};
use moka::future::Cache;
use std::{
    collections::HashMap,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, UNIX_EPOCH},
};
use tokio::task;

#[derive(Clone)]
pub struct TemplateService {
    template_dir: PathBuf,
    env_cache: Cache<String, Arc<CachedEnvironment>>,
}

#[derive(Clone)]
struct CachedEnvironment {
    env: Arc<Environment<'static>>,
    fingerprint: u64,
}

impl TemplateService {
    pub fn new(template_dir: impl Into<PathBuf>) -> Self {
        Self {
            template_dir: template_dir.into(),
            env_cache: Cache::builder().max_capacity(128).build(),
        }
    }

    pub async fn env_for(&self, tenant_slug: &str) -> Result<Arc<Environment<'static>>> {
        let fingerprint = self.scan_fingerprint(tenant_slug).await?;

        if let Some(cached) = self.env_cache.get(tenant_slug).await {
            if cached.fingerprint == fingerprint {
                return Ok(cached.env.clone());
            }
        }

        let templates = self.read_templates(tenant_slug).await?;
        let env = Self::build_environment(templates)?;
        let env = Arc::new(env);

        let cached = Arc::new(CachedEnvironment {
            env: env.clone(),
            fingerprint,
        });

        self.env_cache.insert(tenant_slug.to_string(), cached).await;

        Ok(env)
    }

    async fn scan_fingerprint(&self, tenant_slug: &str) -> Result<u64> {
        let base = self.template_dir.clone();
        let tenant = tenant_slug.to_string();

        task::spawn_blocking(move || {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();

            let shared_root = canonicalize_or(base.join("_shared"));
            let tenant_root = canonicalize_or(base.join(&tenant));

            hasher.write_u64(fingerprint_for(&shared_root)?);
            hasher.write_u64(fingerprint_for(&tenant_root)?);

            Ok::<_, anyhow::Error>(hasher.finish())
        })
        .await
        .context("fingerprint task failed")?
    }

    async fn read_templates(&self, tenant_slug: &str) -> Result<HashMap<String, String>> {
        let base = self.template_dir.clone();
        let tenant = tenant_slug.to_string();

        task::spawn_blocking(move || {
            let shared_root = canonicalize_or(base.join("_shared"));
            let tenant_root = canonicalize_or(base.join(&tenant));

            let mut map = load_templates(&shared_root)?;
            map.extend(load_templates(&tenant_root)?);

            Ok::<_, anyhow::Error>(map)
        })
        .await
        .context("template load task failed")?
    }

    fn build_environment(templates: HashMap<String, String>) -> Result<Environment<'static>> {
        let mut env = Environment::new();

        env.set_auto_escape_callback(|name| {
            if name.ends_with(".html") {
                AutoEscape::Html
            } else {
                AutoEscape::None
            }
        });

        // Disable fuel limit - it was causing serialization truncation
        // We'll rely on other limits for safety
        env.set_fuel(None);

        let loader_map = Arc::new(templates);
        env.set_loader(move |name| {
            loader_map.get(name).cloned().map(Some).ok_or_else(|| {
                Error::new(
                    ErrorKind::TemplateNotFound,
                    format!("template '{name}' not found"),
                )
            })
        });

        env.add_function("now", |_args: &[Value]| {
            Ok(Value::from_serialize(Utc::now()))
        });

        // Add a custom filter to materialize sequences fully
        env.add_filter("materialize", |v: Value| Ok(v));

        Ok(env)
    }
}

fn fingerprint_for(root: &Path) -> Result<u64> {
    if !root.exists() {
        return Ok(0);
    }

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).with_context(|| format!("reading {dir:?}"))? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                stack.push(path);
                continue;
            }

            if !should_include(&path) {
                continue;
            }

            let rel = path.strip_prefix(root).unwrap_or(&path);
            rel.hash(&mut hasher);

            let metadata = entry.metadata()?;
            hasher.write_u64(metadata.len());

            if let Ok(modified) = metadata.modified() {
                let nanos = modified
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_else(|_| Duration::ZERO)
                    .as_nanos();
                hasher.write_u64(nanos as u64);
            }
        }
    }

    Ok(hasher.finish())
}

fn load_templates(root: &Path) -> Result<HashMap<String, String>> {
    if !root.exists() {
        return Ok(HashMap::new());
    }

    let mut map = HashMap::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).with_context(|| format!("reading {dir:?}"))? {
            let entry = entry?;
            let path = entry.path();

            if entry.file_type()?.is_dir() {
                stack.push(path);
                continue;
            }

            if !should_include(&path) {
                continue;
            }

            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");

            let content = fs::read_to_string(&path)
                .with_context(|| format!("loading template {:?}", path.display()))?;

            map.insert(rel, content);
        }
    }

    Ok(map)
}

fn should_include(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| !name.starts_with('.'))
        .unwrap_or(false)
        && path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| matches!(ext, "html" | "jinja" | "j2" | "txt" | "jinja2"))
            .unwrap_or(true)
}

fn canonicalize_or(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}
