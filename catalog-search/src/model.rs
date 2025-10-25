use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catalog {
    pub items: Vec<Furniture>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Furniture {
    pub id: String,
    #[serde(default)]
    pub integration_id: Option<String>,
    #[serde(default)]
    pub integration_type: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub specifications: Option<String>,
    #[serde(default)]
    pub price: Option<f64>,
    #[serde(default)]
    pub weight: Option<f64>,
    #[serde(default)]
    pub sku: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub depth: Option<f64>,
    #[serde(default)]
    pub height: Option<f64>,
    #[serde(default)]
    pub width: Option<f64>,
    #[serde(default)]
    pub description_text: Option<String>,
    #[serde(default)]
    pub quick_description: Option<String>,
    #[serde(default)]
    pub has_variations: Option<bool>,
    #[serde(default)]
    pub quick_specifications: Option<String>,
    #[serde(default)]
    pub priority: Option<i64>,
    #[serde(default)]
    pub is_promotional: Option<bool>,
    #[serde(default)]
    pub promotional_price: Option<f64>,
    #[serde(default)]
    pub variations: Vec<Variation>,
    #[serde(default)]
    pub searchable_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Variation {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub price: Option<f64>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub secondary_color: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub furniture_id: Option<String>,
    #[serde(default)]
    pub order: Option<i64>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub quick_description: Option<String>,
    #[serde(default)]
    pub depth: Option<f64>,
    #[serde(default)]
    pub height: Option<f64>,
    #[serde(default)]
    pub weight: Option<f64>,
    #[serde(default)]
    pub width: Option<f64>,
    #[serde(default)]
    pub quick_specifications: Option<String>,
    #[serde(default)]
    pub size: Option<String>,
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub is_promotional: Option<bool>,
    #[serde(default)]
    pub promotional_price: Option<f64>,
}

impl Catalog {
    pub fn empty() -> Self {
        Self { items: Vec::new() }
    }
}
