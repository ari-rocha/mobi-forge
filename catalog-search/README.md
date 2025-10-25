# Catalog Search WASM Module

This crate compiles the furniture and variation datasets into a compact
[`bincode`](https://docs.rs/bincode/latest/bincode/) blob that ships inside the
WebAssembly binary. The browser consumes the module to provide a completely
offline catalog and search experience.

## Building

1. Install [`wasm-pack`](https://rustwasm.github.io/wasm-pack/installer/) if it
   is not available yet.
2. From the crate directory run:

   ```bash
   cd catalog-search
   wasm-pack build --target web --out-dir ../static/pkg
   ```

3. The command generates `static/pkg/catalog_search.js` and the corresponding
   `.wasm` binary. The search templates import the module via
   `/static/catalog-search-app.js`.

The build script automatically reads the source data from
`commerce-data/Furniture.json` and `commerce-data/Variation.json`, joins the
records, and precomputes the text used for ranking search results.

## Development Tips

- Re-run the `wasm-pack build` command whenever the source CSV/JSON files
  change. The loader script performs a simple runtime check and will surface an
  error banner if the WASM bundle is missing.
- The generated files are ignored by git via `static/.gitignore` to avoid
  committing large binaries.
