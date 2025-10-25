const loaderScript =
  document.currentScript ||
  document.querySelector('script[data-module="catalog-search"]');

async function bootstrap() {
  const siteSlug = loaderScript?.dataset?.siteSlug ?? "store";
  const view = loaderScript?.dataset?.view ?? "search";
  const resultsTarget =
    loaderScript?.dataset?.resultsTarget ?? "#search-results";
  const summaryTarget =
    loaderScript?.dataset?.summaryTarget ?? "#search-summary";
  const emptyStateTarget =
    loaderScript?.dataset?.emptyTarget ?? "#search-empty";
  const formSelector = loaderScript?.dataset?.formSelector ?? ".search-form";
  const inputSelector = loaderScript?.dataset?.inputSelector ?? "#search-query";

  let wasmModule;
  try {
    wasmModule = await import("./pkg/catalog_search.js");
  } catch (error) {
    console.error("Failed to load catalog_search.js", error);
    showStaticError(
      emptyStateTarget,
      "Unable to load the catalog search engine."
    );
    return;
  }

  const initWasm = wasmModule.default;
  const CatalogSearch = wasmModule.CatalogSearch;

  if (typeof initWasm !== "function" || typeof CatalogSearch !== "function") {
    console.error("catalog_search.js did not expose the expected exports");
    showStaticError(
      emptyStateTarget,
      "Catalog search module is incomplete. Rebuild the WASM package."
    );
    return;
  }

  try {
    await initWasm();
  } catch (error) {
    console.error("Failed to initialise WASM module", error);
    showStaticError(
      emptyStateTarget,
      "Unable to initialise the catalog search engine."
    );
    return;
  }

  let engine;
  try {
    engine = new CatalogSearch();
  } catch (error) {
    console.error("Failed to construct CatalogSearch", error);
    showStaticError(
      emptyStateTarget,
      "Catalog search engine is not available."
    );
    return;
  }

  let allItems;
  try {
    allItems = engine.all();
  } catch (error) {
    console.error("CatalogSearch::all failed", error);
    showStaticError(emptyStateTarget, "Unable to load the local catalog data.");
    return;
  }

  if (!Array.isArray(allItems)) {
    console.error("CatalogSearch::all did not return an array", allItems);
    showStaticError(
      emptyStateTarget,
      "Catalog data is not in the expected format."
    );
    return;
  }

  if (view === "catalog") {
    renderProductGrid(resultsTarget, allItems, siteSlug);
    hideElement(emptyStateTarget);
    return;
  }

  const searchForm = document.querySelector(formSelector);
  const searchInput = document.querySelector(inputSelector);
  const resultsContainer = document.querySelector(resultsTarget);

  if (!searchForm || !searchInput || !resultsContainer) {
    console.warn(
      "Search form or result container missing, skipping WASM search wiring."
    );
    return;
  }

  const summaryNode = document.querySelector(summaryTarget);
  const emptyNode = document.querySelector(emptyStateTarget);

  let lastQuery = null;

  const runSearch = (term, options = {}) => {
    const { updateHistory = true } = options;
    const query = term.trim();

    if (!query) {
      lastQuery = "";
      resultsContainer.innerHTML = "";
      if (emptyNode) {
        emptyNode.textContent =
          "Enter a search term to find products in our catalog.";
        emptyNode.hidden = false;
      }
      if (summaryNode) {
        summaryNode.textContent = "";
      }
      if (updateHistory) {
        updateUrl("");
      }
      return;
    }

    if (query === lastQuery) {
      return;
    }

    let matches;
    try {
      matches = engine.search(query);
    } catch (error) {
      console.error("CatalogSearch::search failed", error);
      showStaticError(
        emptyStateTarget,
        "Search failed. Refresh the page and try again."
      );
      return;
    }

    if (!Array.isArray(matches)) {
      console.error("Search results not an array", matches);
      showStaticError(
        emptyStateTarget,
        "Search results are not in the expected format."
      );
      return;
    }

    lastQuery = query;

    if (matches.length === 0) {
      resultsContainer.innerHTML = "";
      if (emptyNode) {
        emptyNode.textContent = `We couldn't find anything for "${query}". Try a different search term.`;
        emptyNode.hidden = false;
      }
      if (summaryNode) {
        summaryNode.textContent = `0 results for "${query}"`;
      }
      if (updateHistory) {
        updateUrl(query);
      }
      return;
    }

    renderProductGrid(resultsContainer, matches, siteSlug);
    if (summaryNode) {
      const label = matches.length === 1 ? "result" : "results";
      summaryNode.textContent = `${matches.length} ${label} for "${query}"`;
    }
    if (emptyNode) {
      emptyNode.hidden = true;
    }
    if (updateHistory) {
      updateUrl(query);
    }
  };

  searchForm.addEventListener("submit", (event) => {
    event.preventDefault();
    runSearch(searchInput.value || "");
  });

  const debouncedSearch = debounce((value) => {
    runSearch(value ?? "");
  }, 150);

  searchInput.addEventListener("input", () => {
    runSearch(searchInput.value || "");
  });

  searchInput.addEventListener("search", () => {
    runSearch(searchInput.value || "");
  });

  const initialQuery =
    searchInput.value ||
    new URLSearchParams(window.location.search).get("q") ||
    "";
  if (initialQuery) {
    searchInput.value = initialQuery;
    runSearch(initialQuery, { updateHistory: false });
  }
}

function renderProductGrid(containerSelector, products, siteSlug) {
  const container =
    typeof containerSelector === "string"
      ? document.querySelector(containerSelector)
      : containerSelector;

  if (!container) {
    console.warn("Result container not found", containerSelector);
    return;
  }

  container.innerHTML = "";
  container.classList.add("product-grid");

  if (!Array.isArray(products) || products.length === 0) {
    const empty = document.createElement("p");
    empty.className = "product-grid__empty";
    empty.textContent = "No products available yet.";
    container.appendChild(empty);
    return;
  }

  for (const product of products) {
    const card = buildProductCard(product, siteSlug);
    container.appendChild(card);
  }
}

function buildProductCard(product, siteSlug) {
  const article = document.createElement("article");
  article.className = "product-card";

  const body = document.createElement("div");
  body.className = "product-card__body";
  article.appendChild(body);

  const title = document.createElement("h3");
  const productTitle =
    product.name ||
    product.quick_description ||
    product.slug ||
    "Untitled product";
  const slug = product.slug || product.id;

  if (slug) {
    const link = document.createElement("a");
    link.href = `/@${siteSlug}/products/${encodeURIComponent(slug)}`;
    link.textContent = productTitle;
    title.appendChild(link);
  } else {
    title.textContent = productTitle;
  }
  body.appendChild(title);

  if (product.quick_description) {
    const description = document.createElement("p");
    description.textContent = truncate(product.quick_description, 160);
    body.appendChild(description);
  }

  const priceNode = document.createElement("p");
  priceNode.className = "product-card__price";
  priceNode.textContent = formatPrice(product);
  body.appendChild(priceNode);

  if (Array.isArray(product.variations) && product.variations.length > 0) {
    const badgeList = document.createElement("ul");
    badgeList.className = "badge-list";

    for (const variation of product.variations.slice(0, 6)) {
      const badgeLabel =
        variation.name ||
        variation.quick_description ||
        variation.size ||
        variation.color ||
        null;
      if (!badgeLabel) {
        continue;
      }
      const badge = document.createElement("li");
      badge.className = "badge";
      badge.textContent = badgeLabel;
      badgeList.appendChild(badge);
    }

    if (badgeList.childElementCount > 0) {
      body.appendChild(badgeList);
    }
  }

  return article;
}

function truncate(text, limit) {
  if (typeof text !== "string") {
    return "";
  }
  if (text.length <= limit) {
    return text;
  }
  return `${text.slice(0, limit)}…`;
}

function formatPrice(product) {
  const promo = normaliseNumber(product.promotional_price);
  const price = normaliseNumber(product.price);

  if (product.is_promotional && promo !== null) {
    const regular = price !== null ? ` (Regular: ${formatNumber(price)})` : "";
    return `${formatNumber(promo)}${regular}`;
  }

  if (price !== null) {
    return formatNumber(price);
  }

  return "-";
}

function formatNumber(value) {
  return value.toLocaleString(undefined, {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
}

function normaliseNumber(value) {
  if (typeof value === "number" && !Number.isNaN(value)) {
    return value;
  }
  if (typeof value === "string" && value.trim() !== "") {
    const parsed = Number(value);
    return Number.isNaN(parsed) ? null : parsed;
  }
  return null;
}

function updateUrl(query) {
  if (!window.history?.replaceState) {
    return;
  }
  const url = new URL(window.location.href);
  if (query) {
    url.searchParams.set("q", query);
  } else {
    url.searchParams.delete("q");
  }
  window.history.replaceState({}, document.title, url.toString());
}

function showStaticError(selector, message) {
  const target =
    typeof selector === "string" ? document.querySelector(selector) : selector;
  if (!target) {
    console.warn("Error node not found", selector);
    return;
  }
  target.hidden = false;
  target.textContent = message;
}

function hideElement(selector) {
  const node =
    typeof selector === "string" ? document.querySelector(selector) : selector;
  if (node) {
    node.hidden = true;
  }
}

function debounce(fn, delay = 150) {
  let timerId;
  let lastArgs = [];
  return function debounced(...args) {
    lastArgs = args;
    if (timerId) {
      clearTimeout(timerId);
    }
    timerId = setTimeout(() => {
      timerId = undefined;
      fn(...lastArgs);
    }, delay);
  };
}

bootstrap().catch((error) => {
  console.error("Unexpected error while bootstrapping catalog search", error);
});
