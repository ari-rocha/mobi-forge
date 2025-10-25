# Catalog Tools

Utility commands for assembling and testing catalog datasets.

## Commands

### Generate Mock Data

Create a large synthetic dataset (e.g. 200k products) and write a bincode blob:

```bash
cargo run --manifest-path catalog-tools/Cargo.toml -- \
  mock \
  --count 200000 \
  --variations-per-product 4 \
  --catalog-out ./static/catalog.bin
```

Optional flags:

- `--json-out path/to/catalog.json` – dump the generated catalog as JSON for
  inspection.
- `--seed 1234` – make the random generator deterministic.

### Convert JSON Exports

Build the catalog from existing `Furniture.json` / `Variation.json` exports:

```bash
cargo run --manifest-path catalog-tools/Cargo.toml -- \
  from-json \
  --furniture commerce-data/Furniture.json \
  --variations commerce-data/Variation.json \
  --catalog-out static/catalog.bin \
  --json-out commerce-data/catalog.json
```

Both commands automatically compute the searchable text payload used by the
WASM module, so no additional processing is required on the client.
