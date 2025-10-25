use std::{collections::HashMap, env, fs, path::PathBuf};

#[path = "src/model.rs"]
mod model;

use anyhow::{Context, Result};
use bincode::Options;
use model::{Catalog, Furniture, Variation};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawFurniture {
    id: String,
    #[serde(default)]
    integration_id: Option<String>,
    #[serde(default)]
    integration_type: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    slug: Option<String>,
    #[serde(default)]
    specifications: Option<String>,
    #[serde(default)]
    price: Option<f64>,
    #[serde(default)]
    weight: Option<f64>,
    #[serde(default)]
    sku: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    updated_at: Option<String>,
    #[serde(default)]
    project_id: Option<String>,
    #[serde(default)]
    depth: Option<f64>,
    #[serde(default)]
    height: Option<f64>,
    #[serde(default)]
    width: Option<f64>,
    #[serde(default)]
    description: Option<Value>,
    #[serde(default)]
    quick_description: Option<String>,
    #[serde(default)]
    has_variations: Option<bool>,
    #[serde(default)]
    quick_specifications: Option<String>,
    #[serde(default)]
    priority: Option<i64>,
    #[serde(default)]
    is_promotional: Option<bool>,
    #[serde(default)]
    promotional_price: Option<f64>,
}

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=../commerce-data/Furniture.json");
    println!("cargo:rerun-if-changed=../commerce-data/Variation.json");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let furniture_path = manifest_dir.join("../commerce-data/Furniture.json");
    let variation_path = manifest_dir.join("../commerce-data/Variation.json");

    let furniture_raw = fs::read_to_string(&furniture_path)
        .with_context(|| format!("reading {}", furniture_path.display()))?;
    let variation_raw = fs::read_to_string(&variation_path)
        .with_context(|| format!("reading {}", variation_path.display()))?;

    let raw_furnitures: Vec<RawFurniture> =
        serde_json::from_str(&furniture_raw).context("parsing furniture json")?;
    let mut furnitures: Vec<Furniture> = raw_furnitures
        .into_iter()
        .map(RawFurniture::into_furniture)
        .collect();
    let variations: Vec<Variation> =
        serde_json::from_str(&variation_raw).context("parsing variation json")?;

    let mut grouped: HashMap<String, Vec<Variation>> = HashMap::new();

    for variation in variations {
        match variation.furniture_id.clone() {
            Some(furniture_id) if !furniture_id.is_empty() => {
                grouped.entry(furniture_id).or_default().push(variation);
            }
            _ => {
                // Ignore variations without furniture linkage to avoid polluting the catalog.
            }
        }
    }

    for furniture in &mut furnitures {
        if let Some(items) = grouped.remove(&furniture.id) {
            furniture.variations = items;
        }
    }

    // Pre-compute lowercase searchable strings to speed up wasm-side lookups.
    for furniture in &mut furnitures {
        let mut searchable_parts: Vec<String> = Vec::new();
        if let Some(name) = furniture.name.as_ref() {
            searchable_parts.push(name.to_lowercase());
        }
        if let Some(slug) = furniture.slug.as_ref() {
            searchable_parts.push(slug.to_lowercase());
        }
        if let Some(description) = furniture.quick_description.as_ref() {
            searchable_parts.push(description.to_lowercase());
        }
        if let Some(description) = furniture.description_text.as_ref() {
            searchable_parts.push(description.to_lowercase());
        }
        if let Some(specs) = furniture.quick_specifications.as_ref() {
            searchable_parts.push(specs.to_lowercase());
        }
        if let Some(specs) = furniture.specifications.as_ref() {
            searchable_parts.push(specs.to_lowercase());
        }

        for variation in &furniture.variations {
            if let Some(name) = variation.name.as_ref() {
                searchable_parts.push(name.to_lowercase());
            }
            if let Some(desc) = variation.quick_description.as_ref() {
                searchable_parts.push(desc.to_lowercase());
            }
            if let Some(specs) = variation.quick_specifications.as_ref() {
                searchable_parts.push(specs.to_lowercase());
            }
        }

        if furniture.searchable_text.is_empty() && !searchable_parts.is_empty() {
            furniture.searchable_text = searchable_parts.join(" ");
        }
    }

    let catalog = Catalog { items: furnitures };

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let bin_path = out_dir.join("catalog.bin");

    let encoded = bincode::options()
        .with_fixint_encoding()
        .serialize(&catalog)
        .context("encoding catalog with bincode")?;

    fs::write(&bin_path, encoded).with_context(|| format!("writing {}", bin_path.display()))?;

    Ok(())
}

impl RawFurniture {
    fn into_furniture(self) -> Furniture {
        let description_text = self.description.and_then(flatten_description);

        Furniture {
            id: self.id,
            integration_id: self.integration_id,
            integration_type: self.integration_type,
            name: self.name,
            slug: self.slug,
            specifications: self.specifications,
            price: self.price,
            weight: self.weight,
            sku: self.sku,
            status: self.status,
            created_at: self.created_at,
            updated_at: self.updated_at,
            project_id: self.project_id,
            depth: self.depth,
            height: self.height,
            width: self.width,
            description_text,
            quick_description: self.quick_description,
            has_variations: self.has_variations,
            quick_specifications: self.quick_specifications,
            priority: self.priority,
            is_promotional: self.is_promotional,
            promotional_price: self.promotional_price,
            variations: Vec::new(),
            searchable_text: String::new(),
        }
    }
}

fn flatten_description(value: Value) -> Option<String> {
    let mut parts = Vec::new();
    collect_text(&value, &mut parts);
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

fn collect_text(value: &Value, acc: &mut Vec<String>) {
    match value {
        Value::String(text) => {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                acc.push(trimmed.to_string());
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_text(item, acc);
            }
        }
        Value::Object(map) => {
            for item in map.values() {
                collect_text(item, acc);
            }
        }
        _ => {}
    }
}
