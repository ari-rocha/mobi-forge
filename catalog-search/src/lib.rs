pub mod model;

use crate::model::{Catalog, Furniture};
use bincode::Options;
use serde::Serialize;
use serde_wasm_bindgen::Serializer;
use wasm_bindgen::prelude::*;

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
    pub fn new(bytes: &[u8]) -> Result<CatalogSearch, JsValue> {
        let mut catalog = decode_catalog(bytes).map_err(to_js_error)?;
        prepare_catalog(&mut catalog);
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
            compare_priority(a.priority, b.priority).then_with(|| compare_name(&a.name, &b.name))
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

pub fn encode_catalog(catalog: &Catalog) -> bincode::Result<Vec<u8>> {
    bincode::options().with_fixint_encoding().serialize(catalog)
}

pub fn decode_catalog(bytes: &[u8]) -> bincode::Result<Catalog> {
    bincode::options().with_fixint_encoding().deserialize(bytes)
}

pub fn prepare_catalog(catalog: &mut Catalog) {
    for furniture in &mut catalog.items {
        if furniture.searchable_text.trim().is_empty() {
            furniture.searchable_text = build_searchable_text(furniture);
        }
    }
}

fn build_searchable_text(furniture: &Furniture) -> String {
    let mut parts: Vec<String> = Vec::new();
    push_lower(&mut parts, furniture.name.as_deref());
    push_lower(&mut parts, furniture.slug.as_deref());
    push_lower(&mut parts, furniture.description_text.as_deref());
    push_lower(&mut parts, furniture.quick_description.as_deref());
    push_lower(&mut parts, furniture.quick_specifications.as_deref());
    push_lower(&mut parts, furniture.specifications.as_deref());

    for variation in &furniture.variations {
        push_lower(&mut parts, variation.name.as_deref());
        push_lower(&mut parts, variation.quick_description.as_deref());
        push_lower(&mut parts, variation.quick_specifications.as_deref());
        push_lower(&mut parts, variation.color.as_deref());
        push_lower(&mut parts, variation.secondary_color.as_deref());
    }

    parts.join(" ")
}

fn push_lower(parts: &mut Vec<String>, value: Option<&str>) {
    if let Some(text) = value {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            parts.push(trimmed.to_lowercase());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Catalog, Furniture, Variation};

    fn sample_catalog() -> Catalog {
        Catalog {
            items: vec![Furniture {
                id: "1".to_string(),
                name: Some("Sample Chair".into()),
                slug: Some("sample-chair".into()),
                description_text: Some("A comfy chair for reading".into()),
                quick_description: Some("Comfy reading chair".into()),
                quick_specifications: Some("Leather; Walnut".into()),
                price: Some(199.0),
                variations: vec![Variation {
                    id: "v1".into(),
                    name: Some("Walnut".into()),
                    color: Some("Brown".into()),
                    ..Default::default()
                }],
                ..Default::default()
            }],
        }
    }

    #[test]
    fn encode_decode_roundtrip() {
        let catalog = sample_catalog();
        let bytes = encode_catalog(&catalog).expect("encode");
        let decoded = decode_catalog(&bytes).expect("decode");
        assert_eq!(decoded.items.len(), 1);
    }

    #[test]
    fn prepare_catalog_builds_searchable_text() {
        let mut catalog = sample_catalog();
        catalog.items[0].searchable_text.clear();
        prepare_catalog(&mut catalog);
        assert!(!catalog.items[0].searchable_text.is_empty());
    }
}
