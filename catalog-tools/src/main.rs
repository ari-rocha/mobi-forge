use anyhow::{Context, Result};
use catalog_search::{
    encode_catalog,
    model::{Catalog, Furniture, Variation},
    prepare_catalog,
};
use clap::{Args, Parser, Subcommand};
use rand::{
    Rng, SeedableRng,
    distributions::{Alphanumeric, DistString},
    rngs::StdRng,
    seq::SliceRandom,
};
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::HashMap,
    fs,
    io::BufWriter,
    path::{Path, PathBuf},
};
use uuid::Uuid;

#[derive(Parser)]
#[command(
    name = "catalog-tools",
    about = "Utilities for building catalog datasets"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate a synthetic catalog with mock products
    Mock(MockArgs),
    /// Convert JSON furniture/variation exports into a catalog binary
    #[command(name = "from-json")]
    FromJson(FromJsonArgs),
}

#[derive(Args)]
struct MockArgs {
    /// Number of products to generate
    #[arg(long, default_value_t = 10_000)]
    count: usize,
    /// Number of variations per product
    #[arg(long, default_value_t = 3)]
    variations_per_product: usize,
    /// Output path for the catalog bincode blob
    #[arg(long)]
    catalog_out: PathBuf,
    /// Optional path to write the generated catalog as JSON (for inspection)
    #[arg(long)]
    json_out: Option<PathBuf>,
    /// Optional RNG seed to make generation deterministic
    #[arg(long)]
    seed: Option<u64>,
}

#[derive(Args)]
struct FromJsonArgs {
    /// Furniture JSON export (array of products)
    #[arg(long)]
    furniture: PathBuf,
    /// Variation JSON export (array of variations)
    #[arg(long)]
    variations: PathBuf,
    /// Output path for the catalog bincode blob
    #[arg(long)]
    catalog_out: PathBuf,
    /// Optional path to write the derived catalog as JSON (for inspection)
    #[arg(long)]
    json_out: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Mock(args) => run_mock(args),
        Command::FromJson(args) => run_from_json(args),
    }
}

fn run_mock(args: MockArgs) -> Result<()> {
    let mut rng = if let Some(seed) = args.seed {
        StdRng::seed_from_u64(seed)
    } else {
        StdRng::from_entropy()
    };

    let adjectives = [
        "Modern", "Cozy", "Elegant", "Vintage", "Sleek", "Rustic", "Minimal", "Premium", "Compact",
        "Bold", "Lux", "Heritage", "Scandi", "Coastal", "Classic",
    ];
    let materials = [
        "Oak",
        "Walnut",
        "Maple",
        "Beech",
        "Ash",
        "Pine",
        "Birch",
        "Bamboo",
        "Steel",
        "Aluminium",
        "Brass",
        "Linen",
        "Leather",
        "Bouclé",
        "Velvet",
    ];
    let product_types = [
        "Sofa",
        "Armchair",
        "Side Table",
        "Coffee Table",
        "Dining Table",
        "Desk",
        "Bed",
        "Bookshelf",
        "Stool",
        "Bench",
        "Media Console",
        "Cabinet",
        "Nightstand",
        "Dresser",
        "Lamp",
        "Wardrobe",
    ];
    let color_names = [
        "Midnight Blue",
        "Forest Green",
        "Terracotta",
        "Sunset Orange",
        "Slate Gray",
        "Ivory",
        "Charcoal",
        "Moss",
        "Blush",
        "Sand",
        "Sage",
        "Mustard",
        "Teal",
    ];

    let mut items = Vec::with_capacity(args.count);

    for idx in 0..args.count {
        let id = Uuid::new_v4().to_string();
        let adjective = adjectives.choose(&mut rng).unwrap();
        let material = materials.choose(&mut rng).unwrap();
        let product_type = product_types.choose(&mut rng).unwrap();
        let name = format!("{adjective} {material} {product_type}");
        let slug = slugify(&name);
        let price = round_currency(rng.gen_range(50.0..5000.0));
        let promo = rng.gen_bool(0.2);
        let promotional_price = if promo {
            Some(round_currency(price * rng.gen_range(0.7..0.95)))
        } else {
            None
        };

        let mut variations = Vec::with_capacity(args.variations_per_product);
        for variant_idx in 0..args.variations_per_product {
            let variant_id = Uuid::new_v4().to_string();
            let color = color_names.choose(&mut rng).unwrap();
            let secondary_color = color_names.choose(&mut rng).unwrap();
            let variant_price = if rng.gen_bool(0.3) {
                Some(round_currency(price * rng.gen_range(0.9..1.1)))
            } else {
                None
            };

            variations.push(Variation {
                id: variant_id,
                name: Some(format!("{color} Finish")),
                price: variant_price,
                color: Some(color.to_string()),
                secondary_color: Some(secondary_color.to_string()),
                furniture_id: Some(id.clone()),
                order: Some(variant_idx as i64),
                quick_description: Some(format!("{color} accent with {secondary_color} details.")),
                quick_specifications: Some(random_specs(&mut rng)),
                size: Some(format!(
                    "{}cm x {}cm x {}cm",
                    rng.gen_range(30..240),
                    rng.gen_range(30..240),
                    rng.gen_range(30..240)
                )),
                is_promotional: Some(promo && rng.gen_bool(0.5)),
                promotional_price: promotional_price,
                ..Default::default()
            });
        }

        let product_type_lower = product_type.to_lowercase();
        let material_lower = material.to_lowercase();
        let adjective_lower = adjective.to_lowercase();

        let mut features = vec![
            format!("Crafted from premium {material} veneer"),
            format!(
                "Designed for contemporary {product_type} settings",
                product_type = product_type_lower
            ),
            format!("Supports up to {} kg", rng.gen_range(80..320)),
        ];
        if rng.gen_bool(0.4) {
            features.push("Sustainably sourced materials".to_string());
        }
        if rng.gen_bool(0.4) {
            features.push("Tool-free assembly under 20 minutes".to_string());
        }

        let description = format!(
            "{name} blends {material} textures with a {adjective} silhouette. \
            Ideal for living spaces that need a statement piece without compromising comfort. \
            Finished by hand for a unique patina on every unit.",
            material = material_lower,
            adjective = adjective_lower
        );

        let quick_description = format!(
            "{adjective} {product_type} crafted in {material} with refined detailing.",
            adjective = adjective_lower,
            product_type = product_type_lower,
            material = material_lower
        );

        let mut furniture = Furniture {
            id,
            name: Some(name.clone()),
            slug: Some(if idx % 2 == 0 {
                slug.clone()
            } else {
                format!(
                    "{slug}-{}",
                    Alphanumeric.sample_string(&mut rng, 4).to_lowercase()
                )
            }),
            description_text: Some(description),
            quick_description: Some(quick_description),
            quick_specifications: Some(features.join(" | ")),
            price: Some(price),
            has_variations: Some(!variations.is_empty()),
            variations,
            priority: Some((idx % 100) as i64),
            is_promotional: Some(promo),
            promotional_price,
            weight: Some(rng.gen_range(5.0..120.0)),
            depth: Some(rng.gen_range(30.0..120.0)),
            height: Some(rng.gen_range(35.0..210.0)),
            width: Some(rng.gen_range(30.0..240.0)),
            ..Default::default()
        };

        furniture.searchable_text.clear();
        items.push(furniture);
    }

    let mut catalog = Catalog { items };
    prepare_catalog(&mut catalog);
    write_outputs(&catalog, &args.catalog_out, args.json_out.as_deref())?;

    println!(
        "Generated mock catalog with {} products -> {}",
        catalog.items.len(),
        args.catalog_out.display()
    );
    Ok(())
}

fn run_from_json(args: FromJsonArgs) -> Result<()> {
    let furniture_raw = fs::read_to_string(&args.furniture)
        .with_context(|| format!("reading {}", args.furniture.display()))?;
    let variations_raw = fs::read_to_string(&args.variations)
        .with_context(|| format!("reading {}", args.variations.display()))?;

    let raw_furnitures: Vec<RawFurniture> =
        serde_json::from_str(&furniture_raw).context("parsing furniture json")?;
    let mut variations: Vec<Variation> =
        serde_json::from_str(&variations_raw).context("parsing variations json")?;

    let mut by_furniture: HashMap<String, Vec<Variation>> = HashMap::new();
    for variation in variations.drain(..) {
        if let Some(furniture_id) = variation.furniture_id.clone() {
            by_furniture
                .entry(furniture_id)
                .or_default()
                .push(variation);
        }
    }

    let mut items = Vec::with_capacity(raw_furnitures.len());
    for raw in raw_furnitures {
        let mut furniture = raw.into_furniture();
        if let Some(attached) = by_furniture.remove(&furniture.id) {
            furniture.has_variations = Some(!attached.is_empty());
            furniture.variations = attached;
        }
        items.push(furniture);
    }

    let mut catalog = Catalog { items };
    prepare_catalog(&mut catalog);
    write_outputs(&catalog, &args.catalog_out, args.json_out.as_deref())?;

    println!(
        "Built catalog from JSON ({} products) -> {}",
        catalog.items.len(),
        args.catalog_out.display()
    );
    Ok(())
}

fn write_outputs(catalog: &Catalog, catalog_path: &Path, json_path: Option<&Path>) -> Result<()> {
    let bytes = encode_catalog(catalog).context("encoding catalog to bincode")?;
    fs::write(catalog_path, bytes)
        .with_context(|| format!("writing {}", catalog_path.display()))?;

    if let Some(json_path) = json_path {
        let file = fs::File::create(json_path)
            .with_context(|| format!("creating {}", json_path.display()))?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, catalog)
            .with_context(|| format!("writing {}", json_path.display()))?;
    }

    Ok(())
}

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

impl RawFurniture {
    fn into_furniture(self) -> Furniture {
        let description_text = self.description.and_then(flatten_description);
        Furniture {
            id: self.id,
            integration_id: self.integration_id,
            integration_type: self.integration_type,
            name: self.name,
            slug: self
                .slug
                .map(|s| if s.is_empty() { slugify_fallback() } else { s }),
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

fn slugify(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else if c.is_whitespace() || c == '-' || c == '_' {
                '-'
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn slugify_fallback() -> String {
    format!(
        "item-{}",
        Alphanumeric
            .sample_string(&mut StdRng::from_entropy(), 8)
            .to_lowercase()
    )
}

fn random_specs(rng: &mut StdRng) -> String {
    let specs = [
        "Modular design",
        "Stain-resistant upholstery",
        "Solid wood frame",
        "Commercial grade durability",
        "Hidden storage compartment",
        "Soft-close drawers",
        "Integrated cable management",
        "Sustainably sourced",
        "Adjustable shelving",
        "Easily recycled packaging",
    ];
    let count = rng.gen_range(2..=4);
    specs
        .choose_multiple(rng, count)
        .cloned()
        .collect::<Vec<_>>()
        .join(" • ")
}

fn round_currency(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
