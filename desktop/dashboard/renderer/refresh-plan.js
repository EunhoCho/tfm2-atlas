(function (root, factory) {
  const api = factory();
  if (typeof module === "object" && module.exports) module.exports = api;
  else root.RefreshPlan = api;
}(typeof globalThis !== "undefined" ? globalThis : this, () => {
  function fromScopes(scopes, page) {
    const changed = new Set(scopes || []);
    const analytics = changed.has("ANALYTICS_CHANGED") || changed.has("INDEX_CHANGED");
    const catalog = changed.has("CATALOG_CHANGED");
    const draft = page === "draft" && changed.has("DRAFT_CHANGED");
    return { relevant: analytics || catalog || draft, analytics, catalog, draft };
  }

  return { fromScopes };
}));
