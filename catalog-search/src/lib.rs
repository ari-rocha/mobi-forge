mod model;

use crate::model::{Catalog, Furniture};
use bincode::Options;
use serde::Serialize;
use serde_wasm_bindgen::Serializer;
use wasm_bindgen::prelude::*;

const CATALOG_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/catalog.bin"));

#[wasm_bindgen]
pub struct CatalogSearch {
    catalog: Catalog,
}

#[derive(Debug, Serialize)]
struct ProductResult {
    id: String,
    name: Option<String>,
    slug: Option<String>,
    description: Option<String>,
    quick_description: Option<String>,
    quick_specifications: Option<String>,
    price: Option<f64>,
    is_promotional: Option<bool>,
    promotional_price: Option<f64>,
    priority: Option<i64>,
    variations: Vec<VariationResult>,
    score: f32,
}

#[derive(Debug, Serialize)]
struct VariationResult {
    id: String,
    name: Option<String>,
    price: Option<f64>,
    color: Option<String>,
    secondary_color: Option<String>,
    quick_description: Option<String>,
    quick_specifications: Option<String>,
    is_promotional: Option<bool>,
    promotional_price: Option<f64>,
}

#[wasm_bindgen]
impl CatalogSearch {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<CatalogSearch, JsValue> {
        let catalog: Catalog = bincode::options()
            .with_fixint_encoding()
            .deserialize(CATALOG_BYTES)
            .map_err(to_js_error)?;
        Ok(Self { catalog })
    }

    #[wasm_bindgen(js_name = "all")]
    pub fn all_js(&self) -> Result<JsValue, JsValue> {
        let items = self
            .catalog
            .items
            .iter()
            .map(|furniture| build_result(furniture, 0.0))
            .collect::<Vec<_>>();
        to_js_value(&items)
    }

    #[wasm_bindgen]
    pub fn search(&self, query: &str) -> Result<JsValue, JsValue> {
        let trimmed = query.trim().to_lowercase();

        if trimmed.is_empty() {
            return self.top_by_priority(32);
        }

        let tokens: Vec<&str> = trimmed.split_whitespace().collect();

        if tokens.is_empty() {
            return self.top_by_priority(32);
        }

        let mut matches: Vec<ProductResult> = Vec::new();

        for furniture in &self.catalog.items {
            if furniture.searchable_text.is_empty() {
                continue;
            }

            if let Some(score) = compute_score(furniture, &tokens) {
                matches.push(build_result(furniture, score));
            }
        }

        matches.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| compare_priority(a.priority, b.priority))
                .then_with(|| compare_name(&a.name, &b.name))
        });

        matches.truncate(50);

        to_js_value(&matches)
    }
}

impl CatalogSearch {
    fn top_by_priority(&self, limit: usize) -> Result<JsValue, JsValue> {
        let mut items: Vec<ProductResult> = self
            .catalog
            .items
            .iter()
            .map(|item| build_result(item, priority_score(item.priority)))
            .collect();

        items.sort_by(|a, b| {
            compare_priority(a.priority, b.priority)
                .then_with(|| compare_name(&a.name, &b.name))
        });

        items.truncate(limit);
        to_js_value(&items)
    }
}

fn compute_score(furniture: &Furniture, tokens: &[&str]) -> Option<f32> {
    if tokens.is_empty() {
        return Some(priority_score(furniture.priority));
    }

    let base = furniture.searchable_text.as_str();
    if base.is_empty() {
        return None;
    }

    let mut score = 0.0;

    for token in tokens {
        if !base.contains(token) {
            return None;
        }

        score += 1.0;

        if let Some(name) = furniture.name.as_ref() {
            if name.to_lowercase().contains(token) {
                score += 1.0;
            }
        }

        if let Some(slug) = furniture.slug.as_ref() {
            if slug.to_lowercase().contains(token) {
                score += 0.5;
            }
        }
    }

    score += priority_score(furniture.priority);
    Some(score)
}

fn build_result(furniture: &Furniture, score: f32) -> ProductResult {
    ProductResult {
        id: furniture.id.clone(),
        name: furniture.name.clone(),
        slug: furniture.slug.clone(),
        description: furniture.description_text.clone(),
        quick_description: furniture.quick_description.clone(),
        quick_specifications: furniture.quick_specifications.clone(),
        price: furniture.price,
        is_promotional: furniture.is_promotional,
        promotional_price: furniture.promotional_price,
        priority: furniture.priority,
        variations: furniture
            .variations
            .iter()
            .map(|variation| VariationResult {
                id: variation.id.clone(),
                name: variation.name.clone(),
                price: variation.price,
                color: variation.color.clone(),
                secondary_color: variation.secondary_color.clone(),
                quick_description: variation.quick_description.clone(),
                quick_specifications: variation.quick_specifications.clone(),
                is_promotional: variation.is_promotional,
                promotional_price: variation.promotional_price,
            })
            .collect(),
        score,
    }
}

fn priority_score(priority: Option<i64>) -> f32 {
    priority.map(|value| (-value) as f32).unwrap_or(0.0)
}

fn compare_priority(a: Option<i64>, b: Option<i64>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(x), Some(y)) => x.cmp(&y),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

fn compare_name(a: &Option<String>, b: &Option<String>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(x), Some(y)) => x.to_lowercase().cmp(&y.to_lowercase()),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

fn to_js_value<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    value
        .serialize(&Serializer::json_compatible())
        .map_err(|err| JsValue::from_str(&err.to_string()))
}

fn to_js_error<E: std::fmt::Display>(err: E) -> JsValue {
    JsValue::from_str(&err.to_string())
}

#[cfg(all(test, not(target_arch = "wasm32")))]
fn decode_catalog_for_tests() -> Catalog {
    use bincode::Options;

    let options = bincode::options().with_fixint_encoding();
    let mut de = bincode::de::Deserializer::from_slice(CATALOG_BYTES, options);
    serde_path_to_error::deserialize(&mut de)
        .unwrap_or_else(|err| panic!("bincode decode failed at {}: {}", err.path(), err))
}

#[cfg(test)]
mod tests {
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn catalog_decodes_with_native_bincode() {
        super::decode_catalog_for_tests();
    }
}
