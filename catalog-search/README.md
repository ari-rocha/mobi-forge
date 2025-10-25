# Catalog Search WASM Module

This crate exposes the catalog search engine as a WebAssembly module. The
compiled binary no longer embeds catalog data; instead the client code passes a
`bincode` blob at runtime (typically fetched alongside the HTML).

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
   `/static/catalog-search-app.js`, which fetches the catalog blob and passes it
   into the `CatalogSearch` constructor.

## Development Tips

- Re-run the `wasm-pack build` command whenever the source CSV/JSON files
  change. The loader script will surface an error banner if the WASM bundle
  fails to load.
- Catalog data can be produced with `catalog-tools` (see that crate's README).
- The generated files are ignored by git via `static/.gitignore` to avoid
  committing large binaries.
