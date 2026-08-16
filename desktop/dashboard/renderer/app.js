const state = {
  language: "ko",
  messages: { ko: {}, en: {} },
  live: null,
  preview: null,
  profile: null,
  page: "statistics",
  selectedChampion: null,
  detailTab: "statistics",
  search: "",
  initializedProfile: false,
  previewTimer: null,
  previewRevision: null,
  draft: null,
  draftMode: "our_ban",
  draftSearch: "",
  draftRole: "all",
  sidebarHidden: localStorage.getItem("tfm2.dashboard.sidebarHidden") === "1",
  refreshSequence: 0,
  connectionRetryTimer: null,
};

const $ = (selector) => document.querySelector(selector);
const h = (value) => String(value ?? "").replace(/[&<>"']/g, (character) => ({
  "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
}[character]));
const tr = (key) => state.messages[state.language]?.[key] || state.messages.ko?.[key] || key;
const label = (value) => tr({
  all: "all", tournament: "tournament", solo: "solo", solo_and_tournament: "combined",
  first: "first", second: "second", top: "top", jungle: "jungle", mid: "mid", bot: "bot",
  support: "support", classic: "classic", fearless: "fearless", hard_fearless: "hardFearless", fearless_hard: "hardFearless",
  latest: "latest", waiting_for_career: "engineWaiting", validating_schema: "engineValidating",
  indexing: "engineIndexing", ready: "engineReady", error: "engineError",
  cold: "cacheCold", restored: "cacheRestored", rebuilding: "cacheRebuilding",
  checkpointed: "cacheCheckpointed", complete: "cacheComplete", invalid: "cacheInvalid",
}[value] || value);
const pct = (value) => value == null ? "—" : `${Number(value).toFixed(1)}%`;
const number = (value) => value == null ? "—" : Number(value).toLocaleString(state.language === "ko" ? "ko-KR" : "en-US", { maximumFractionDigits: 1 });

function defaultProfile() {
  return { enabled: true, scope: "solo_and_tournament", region: "all", division: "all", role: "all", patch: "latest", sample: { mode: "auto" }, preset: "classic" };
}

function currentAnalytics() {
  if (!state.live?.connected || state.live.engine_status === "waiting_for_career") return null;
  return state.preview || state.live.analytics || null;
}

function applyStaticTranslations() {
  document.documentElement.lang = state.language;
  document.querySelectorAll("[data-i18n]").forEach((node) => { node.textContent = tr(node.dataset.i18n); });
  document.querySelectorAll("[data-placeholder]").forEach((node) => { node.placeholder = tr(node.dataset.placeholder); });
  document.querySelectorAll("[data-i18n-title]").forEach((node) => {
    const title = tr(node.dataset.i18nTitle);
    node.title = title;
    node.setAttribute("aria-label", title);
  });
  document.body.classList.toggle("sidebar-hidden", state.sidebarHidden);
  $("#sidebar").setAttribute("aria-hidden", String(state.sidebarHidden));
  $("#sidebarReveal").hidden = !state.sidebarHidden;
}

function setSidebarHidden(hidden) {
  state.sidebarHidden = Boolean(hidden);
  localStorage.setItem("tfm2.dashboard.sidebarHidden", state.sidebarHidden ? "1" : "0");
  applyStaticTranslations();
}

function toast(message, error = false) {
  const node = $("#toast");
  node.textContent = message;
  node.classList.toggle("error", error);
  node.hidden = false;
  clearTimeout(toast.timer);
  toast.timer = setTimeout(() => { node.hidden = true; }, 3200);
}

function bridgeErrorText(error) {
  const message = String(error?.message || error || "");
  const [code, ...details] = message.split(":");
  const key = {
    tier_apply_failed: "errorTierApply",
    profile_persistence_failed: "errorProfileSave",
    lock_apply_failed: "errorLockApply",
    mastery_write_failed: "errorMasteryWrite",
    mastery_read_failed: "errorMasteryRead",
    schema_preflight_failed: "errorSchema",
  }[code];
  return `${tr("bridgeError")}: ${key ? tr(key) : code}${details.length ? ` · ${details.join(":").trim()}` : ""}`;
}

const previewRunner = LatestRequest.create({
  execute: (profile) => window.tfm2.request("PREVIEW_TIER_PROFILE", profile),
  apply: (preview) => {
    state.preview = preview;
    state.previewRevision = preview.data_revision;
    render();
  },
  onError: (error) => toast(bridgeErrorText(error), true),
});

function scheduleConnectionRetry() {
  if (state.connectionRetryTimer || state.live?.connected) return;
  state.connectionRetryTimer = setTimeout(() => {
    state.connectionRetryTimer = null;
    if (document.visibilityState === "hidden") {
      scheduleConnectionRetry();
      return;
    }
    refresh({ analytics: true, catalog: true, draft: false });
  }, 1000);
}

function clearConnectionRetry() {
  clearTimeout(state.connectionRetryTimer);
  state.connectionRetryTimer = null;
}

async function refresh({ analytics = true, catalog = true, draft = state.page === "draft" } = {}) {
  // A BrowserWindow starts hidden while index.html loads. The first refresh must
  // still run; only defer later background refreshes after live state exists.
  if (document.visibilityState === "hidden" && state.live) return;
  const sequence = ++state.refreshSequence;
  try {
    const draftRequest = draft
      ? window.tfm2.request("GET_MOCK_DRAFT").catch(() => state.draft)
      : Promise.resolve(state.draft);
    const catalogRequest = catalog
      ? window.tfm2.request("GET_CATALOG").catch(() => state.live?.champion_catalog || { status: "loading", revision: 0, champions: [] })
      : Promise.resolve(state.live?.champion_catalog || { status: "loading", champions: [] });
    const dashboardRequest = analytics || !state.live
      ? window.tfm2.request("GET_DASHBOARD")
      : Promise.resolve(state.live);
    const [live, draftState, catalogState] = await Promise.all([
      dashboardRequest,
      draftRequest,
      catalogRequest,
    ]);
    if (sequence !== state.refreshSequence) return;
    const previousRevision = state.live?.data_revision;
    live.champion_catalog = catalogState;
    live.champion_info = catalogState?.status === "ready" ? catalogState.champions : [];
    state.live = live;
    state.draft = draftState;
    if (live.connected) clearConnectionRetry();
    else scheduleConnectionRetry();
    if (!state.initializedProfile) {
      state.profile = structuredClone(live.preview_profile || live.active_profile || defaultProfile());
      state.initializedProfile = true;
    }
    render();
    if (state.initializedProfile && live.engine_status === "ready" && previousRevision != null && previousRevision !== live.data_revision) {
      schedulePreview();
    }
  } catch (error) {
    state.live = null;
    state.preview = null;
    render();
    scheduleConnectionRetry();
  }
}

function render() {
  applyStaticTranslations();
  renderConnection();
  renderNav();
  renderChampionSidebar();
  renderProfile();
  renderContent();
}

function renderConnection() {
  const connected = Boolean(state.live?.connected && state.live?.engine_status !== "waiting_for_career");
  const node = $("#connection");
  node.classList.toggle("online", connected);
  node.querySelector("strong").textContent = tr(connected ? "connected" : "disconnected");
  node.querySelector("small").textContent = connected
    ? `${label(state.live.engine_status)} · ${state.live.indexed_matches} ${tr("games")}`
    : "TFM2 · 0.5.5";
}

function renderNav() {
  const pages = [["statistics", "overviewStats", "▦"], ["champion", "championInfo", "◈"], ["draft", "draftRecommendation", "♜"], ["about", "about", "ⓘ"]];
  $("#primaryNav").innerHTML = pages.map(([page, key, icon]) => `
    <button class="nav-item ${state.page === page ? "active" : ""}" data-page="${page}"><span>${icon}</span>${h(tr(key))}</button>
  `).join("");
  $("#primaryNav").querySelectorAll("[data-page]").forEach((button) => button.addEventListener("click", () => {
    state.page = button.dataset.page;
    render();
    if (state.page === "draft") refresh();
  }));
  const pageKeys = { statistics: "overviewStats", champion: "championInfo", draft: "draftRecommendation", about: "about" };
  $("#pageTitle").textContent = tr(pageKeys[state.page]);
}

function champions() {
  const activeInfo = state.live?.champion_catalog?.status === "ready"
    ? state.live.champion_catalog.champions
    : [];
  return activeChampionRows(
    mergeChampionRows(currentAnalytics()?.champions || [], activeInfo),
    activeInfo,
  );
}

function championName(championId) {
  const activeInfo = state.live?.champion_catalog?.status === "ready" ? state.live.champion_catalog.champions : [];
  return activeInfo.find((info) => info.champion_id === championId)?.display_name || championId;
}

function renderChampionSidebar() {
  const query = state.search.toLowerCase();
  const list = champions().filter((row) => row.champion_id.toLowerCase().includes(query) || row.display_name.toLowerCase().includes(query));
  $("#championList").innerHTML = list.map((row) => `
    <button class="champion-mini ${state.selectedChampion === row.champion_id ? "active" : ""}" data-champion="${h(row.champion_id)}">
      <span class="champion-avatar">${h(row.display_name.slice(0, 2).toUpperCase())}</span>
      <span><strong>${h(row.display_name)}</strong><small>${tierLabel(row.tier)} · ${row.pick_count}</small></span>
    </button>
  `).join("");
  $("#championList").querySelectorAll("[data-champion]").forEach((button) => button.addEventListener("click", () => {
    state.selectedChampion = button.dataset.champion;
    state.page = "champion";
    render();
  }));
}

function renderProfile() {
  const panel = $("#profilePanel");
  if (!state.live?.connected || state.live.engine_status !== "ready" || state.page !== "statistics") {
    panel.hidden = true;
    return;
  }
  panel.hidden = false;
  const profile = state.profile || defaultProfile();
  const divisionDisabled = profile.scope !== "tournament";
  panel.innerHTML = `
    <div class="profile-head"><div><strong>${h(tr("profile"))}</strong><small>${h(tr("previewNotice"))}</small></div><label class="toggle"><input id="profileEnabled" type="checkbox" ${profile.enabled ? "checked" : ""}><span></span></label></div>
    <div class="profile-grid">
      ${selectField("scope", "scope", [["solo_and_tournament", "combined"], ["tournament", "tournament"], ["solo", "solo"]], profile.scope)}
      ${selectField("region", "region", [["all", "all"], ["kr", "KR"], ["cn", "CN"], ["eu", "EU"], ["na", "NA"], ["sa", "SA"], ["jp", "JP"]], profile.region)}
      ${selectField("division", "division", [["all", "all"], ["first", "first"], ["second", "second"]], profile.division, divisionDisabled)}
      ${selectField("role", "role", [["all", "all"], ["top", "top"], ["jungle", "jungle"], ["mid", "mid"], ["bot", "bot"], ["support", "support"]], profile.role)}
      ${selectField("patch", "patch", [["latest", "latest"], [null, "all"]].concat((state.live.analytics?.available_patches || []).map((patch) => [patch, patch])), profile.patch)}
      ${selectField("preset", "preset", [["classic", "classic"], ["fearless", "fearless"], ["hard_fearless", "hardFearless"]], profile.preset)}
      <label class="field"><span>${h(tr("sample"))}</span><div class="sample-field"><select id="sampleMode"><option value="auto" ${profile.sample?.mode === "auto" ? "selected" : ""}>${h(tr("auto"))}</option><option value="minimum" ${profile.sample?.mode === "minimum" ? "selected" : ""}>${h(tr("minimum"))}</option></select><input id="sampleGames" type="number" min="1" max="10000" value="${profile.sample?.games || 5}" ${profile.sample?.mode !== "minimum" ? "disabled" : ""}></div></label>
    </div>
    <div class="profile-actions"><span class="profile-diff">${profileSummary(profile)}</span><button id="previewProfile" class="secondary">${h(tr("preview"))}</button><button id="applyProfile" class="primary">${h(tr("apply"))}</button></div>
  `;
  bindProfileControls();
}

function selectField(id, title, options, selected, disabled = false) {
  return `<label class="field"><span>${h(tr(title))}</span><select id="${id}" ${disabled ? "disabled" : ""}>${options.map(([value, key]) => `<option value="${value == null ? "" : h(value)}" ${selected === value ? "selected" : ""}>${h(key.length <= 3 && key === key.toUpperCase() ? key : tr(key))}</option>`).join("")}</select></label>`;
}

function bindProfileControls() {
  const update = () => {
    const scope = $("#scope").value;
    state.profile = {
      enabled: $("#profileEnabled").checked,
      scope,
      region: $("#region").value,
      division: scope === "tournament" ? $("#division").value : "all",
      role: $("#role").value,
      patch: $("#patch").value || null,
      sample: $("#sampleMode").value === "minimum" ? { mode: "minimum", games: Math.max(1, Number($("#sampleGames").value) || 1) } : { mode: "auto" },
      preset: $("#preset").value,
    };
  };
  $("#profilePanel").querySelectorAll("select,input").forEach((control) => control.addEventListener("change", () => { update(); renderProfile(); schedulePreview(); }));
  $("#previewProfile").addEventListener("click", async () => {
    update();
    await requestPreview();
  });
  $("#applyProfile").addEventListener("click", async () => {
    update();
    const button = $("#applyProfile");
    button.disabled = true;
    button.textContent = tr("applying");
    try {
      state.live = await window.tfm2.request("APPLY_TIER_PROFILE", state.profile);
      previewRunner.invalidate();
      state.preview = null;
      await window.tfm2.saveProfile(state.profile);
      toast(tr("applied"));
      render();
    } catch (error) { toast(bridgeErrorText(error), true); renderProfile(); }
  });
}

function schedulePreview() {
  clearTimeout(state.previewTimer);
  if (state.live?.engine_status !== "ready") return;
  state.previewTimer = setTimeout(requestPreview, 220);
}

async function requestPreview() {
  const profile = structuredClone(state.profile || defaultProfile());
  return previewRunner.submit(profile);
}

function profileSummary(profile) {
  return [label(profile.scope), profile.region.toUpperCase(), profile.scope === "tournament" ? label(profile.division) : null, label(profile.role), profile.patch ? label(profile.patch) : tr("all")].filter(Boolean).map(h).join(" · ");
}

function renderContent() {
  const content = $("#content");
  if (!state.live?.connected || state.live.engine_status === "waiting_for_career") {
    content.innerHTML = waitingView();
    return;
  }
  if (["validating_schema", "indexing"].includes(state.live.engine_status)) {
    content.innerHTML = loadingView();
    return;
  }
  if (state.page === "statistics") renderStatistics(content);
  else if (state.page === "champion") renderChampion(content);
  else if (state.page === "draft") renderDraft(content);
  else renderAbout(content);
}

function loadingView() {
  const progress = state.live?.index_progress || {};
  const total = Math.max(0, Number(progress.total) || 0);
  const processed = Math.max(0, Number(progress.processed) || 0);
  const percent = total ? Math.min(100, processed / total * 100) : 0;
  return `<div class="waiting loading-view"><div class="waiting-icon spinner">↻</div><h2>${h(tr("indexLoading"))}</h2><p>${h(tr("indexLoadingDetail"))}</p><progress max="100" value="${percent}"></progress><div class="loading-counts"><strong>${processed.toLocaleString()} / ${total.toLocaleString()}</strong><span>${h(tr("indexed"))}: ${number(progress.indexed)} · ${h(tr("failed"))}: ${number(progress.failed)} · ${h(tr("pending"))}: ${number(progress.pending)}</span><span>${h(tr("cacheState"))}: ${h(label(progress.cache_state || "cold"))}</span></div></div>`;
}

function waitingView() {
  return `<div class="waiting"><div class="waiting-icon">⌁</div><h2>${h(tr("disconnected"))}</h2><p>${h(tr("waitingDetail"))}</p><div class="diagnostic-card"><strong>${h(tr("diagnostics"))}</strong><ol><li>tfm2_atlas_core/mod.mod_info</li><li>tfm2_atlas_core/tfm2_atlas_core.dll</li><li>tfm2_atlas_client_055/tfm2_atlas_client_055.dll</li><li>TFM2 · 0.5.5</li></ol></div><button class="secondary" id="retry">${h(tr("refresh"))}</button></div>`;
}

function renderStatistics(content) {
  const analytics = currentAnalytics();
  if (!analytics) { content.innerHTML = `<div class="empty">${h(tr("noData"))}</div>`; return; }
  const rows = champions();
  const eligible = rows.filter((row) => row.eligible).length;
  content.innerHTML = `
    ${tierApplicationBanner()}
    <div class="summary-grid">
      ${summaryCard(tr("indexed"), state.live.indexed_matches, "mint")}${summaryCard(tr("pending"), state.live.pending_records)}${summaryCard(tr("revision"), analytics.data_revision)}${summaryCard(tr("eligible"), `${eligible} / ${rows.length}`, "blue")}
    </div>
    <div class="table-card"><div class="card-heading"><div><strong>${h(tr("overviewStats"))}</strong><small>${h(profileSummary(state.preview ? state.profile : state.live.active_profile))}</small></div><div class="card-actions"><span>${analytics.selected_matches} ${h(tr("games"))}</span><button id="importTierTsv" class="secondary small">${h(tr("importTsv"))}</button><button id="exportTierTsv" class="secondary small">${h(tr("exportTsv"))}</button></div></div>
      <div class="table-scroll"><table><thead><tr><th>${h(tr("rank"))}</th><th>${h(tr("champion"))}</th><th>${h(tr("tier"))}</th><th>${h(tr("score"))}</th><th>${h(tr("bestRole"))}</th><th>${h(tr("winRate"))}</th><th>${h(tr("pickRate"))}</th><th>${h(tr("banRate"))}</th><th>${h(tr("sampleCount"))}</th><th>${h(tr("status"))}</th></tr></thead><tbody>
        ${rows.map((row, index) => championRow(row, index)).join("") || `<tr><td colspan="10">${h(tr("noData"))}</td></tr>`}
      </tbody></table></div>
    </div>`;
  content.querySelectorAll("[data-champion]").forEach((row) => row.addEventListener("click", () => { state.selectedChampion = row.dataset.champion; state.page = "champion"; render(); }));
  $("#exportTierTsv").addEventListener("click", exportTierTsv);
  $("#importTierTsv").addEventListener("click", importTierTsv);
}

function tierApplicationBanner() {
  const status = state.live?.tier_application;
  if (!status) return "";
  const serverOk = status.applied && status.readback_verified;
  const screenOk = serverOk && status.client_screen_verified;
  const stateClass = screenOk ? "ok" : serverOk ? "pending" : "error";
  const title = screenOk ? tr("tierApplied") : serverOk ? tr("tierScreenPending") : tr("tierNotApplied");
  const detail = serverOk
    ? `${h(tr("team"))} ${status.team_id ?? "—"} · ${number(status.applied_champions)} ${h(tr("champions"))}${screenOk ? "" : ` · ${h(tr("tierScreenPendingDetail"))}`}`
    : h(status.error || tr("notAppliedYet"));
  return `<div class="application-status ${stateClass}"><strong>${h(title)}</strong><span>${detail}</span></div>`;
}

async function exportTierTsv() {
  try {
    const exported = await window.tfm2.request("EXPORT_TIER_TSV");
    const result = await window.tfm2.exportTierTsv(exported.tsv);
    if (result.ok) toast(`${tr("tsvExported")} ${exported.row_count}`);
  } catch (error) {
    toast(bridgeErrorText(error), true);
  }
}

async function importTierTsv() {
  try {
    const imported = await window.tfm2.importTierTsv();
    if (!imported.ok) return;
    const validated = await window.tfm2.request("VALIDATE_TIER_TSV", { tsv: imported.tsv });
    toast(`${tr("tsvValidated")} v${validated.version} · ${validated.row_count}`);
  } catch (error) {
    toast(`${tr("tsvInvalid")}: ${error.message}`, true);
  }
}

function summaryCard(title, value, accent = "") { return `<article class="summary-card ${accent}"><small>${h(title)}</small><strong>${h(value)}</strong></article>`; }
function gamePositions(championId) {
  const champion = (state.live?.champion_info || []).find((row) => row.champion_id === championId);
  return championPositions(champion || {});
}
function gamePositionText(championId) {
  const roles = gamePositions(championId);
  return roles.length ? roles.map(label).join(" · ") : tr("roleUnknown");
}
function championRow(row, index) {
  return `<tr data-champion="${h(row.champion_id)}"><td>${index + 1}</td><td><div class="champion-cell"><span class="champion-avatar">${h(row.display_name.slice(0, 2).toUpperCase())}</span><strong>${h(row.display_name)}</strong></div></td><td><span class="tier tier-${h(row.tier)}">${tierLabel(row.tier)}</span></td><td>${row.overall == null ? "—" : Number(row.overall).toFixed(1)}</td><td>${h(gamePositionText(row.champion_id))}</td><td>${pct(row.win_rate)}</td><td>${pct(row.pick_rate)}</td><td>${pct(row.ban_rate)}</td><td>${row.pick_count}</td><td><span class="eligibility ${row.eligible ? "yes" : "no"}">${h(tr(row.eligible ? "eligible" : "notEligible"))}</span></td></tr>`;
}
function tierLabel(tier) { return ({ op: "OP", one: "1", two: "2", three: "3", four: "4", no_tier: "—" })[tier] || "—"; }

function renderChampion(content) {
  const rows = champions();
  if (!state.selectedChampion || !rows.some((row) => row.champion_id === state.selectedChampion)) state.selectedChampion = rows[0]?.champion_id || null;
  const row = rows.find((candidate) => candidate.champion_id === state.selectedChampion);
  if (!row) { content.innerHTML = `<div class="empty">${h(tr("noData"))}</div>`; return; }
  const info = state.live.champion_info?.find((candidate) => candidate.champion_id === row.champion_id);
  const tabs = [["statistics", "statistics"], ["basic", "basicInfo"], ["patch", "patchHistory"], ["relations", "relationsBuild"]];
  content.innerHTML = `<section class="champion-header"><span class="champion-avatar large">${h(row.display_name.slice(0, 2).toUpperCase())}</span><div><small>${tierLabel(row.tier)} TIER · ${row.pick_count} ${h(tr("games"))}</small><h2>${h(row.display_name)}</h2><p>${h(info?.tags?.join(" · ") || tr(row.eligible ? "eligible" : "notEligible"))}</p></div><strong class="hero-score">${row.overall == null ? "—" : Number(row.overall).toFixed(1)}</strong></section>
    <nav class="detail-tabs">${tabs.map(([tab, key]) => `<button class="${state.detailTab === tab ? "active" : ""}" data-detail="${tab}">${h(tr(key))}</button>`).join("")}</nav><section class="detail-body">${championTab(row, info)}</section>`;
  content.querySelectorAll("[data-detail]").forEach((button) => button.addEventListener("click", () => { state.detailTab = button.dataset.detail; renderChampion(content); }));
}

function championTab(row, info) {
  if (state.detailTab === "statistics") return `<div class="metric-grid">${metric(tr("winRate"), pct(row.win_rate))}${metric(tr("pickRate"), pct(row.pick_rate))}${metric(tr("banRate"), pct(row.ban_rate))}${metric(tr("averageDamage"), number(row.average_dealt))}${metric(tr("averageTank"), number(row.average_taken))}${metric(tr("averageHeal"), number(row.average_healing))}${metric(tr("bestRole"), gamePositionText(row.champion_id))}</div>${championRoleStats(row.role_profile)}${components(row)}`;
  if (state.detailTab === "basic") return info ? `<div class="two-column"><article class="panel"><h3>${h(tr("baseStats"))}</h3>${statList(info.stat)}</article><article class="panel"><h3>${h(tr("growthStats"))}</h3>${statList(info.growth)}</article></div><article class="panel inline"><strong>${h(tr("category"))}</strong><span>${h(info.category || "—")}</span><strong>${h(tr("tags"))}</strong><span>${h(info.tags?.join(", ") || "—")}</span></article>` : `<div class="empty">${h(tr("waitingDetail"))}</div>`;
  if (state.detailTab === "patch") return patchHistory(row);
  return `<div class="three-column"><article class="panel"><h3>${h(tr("matchup"))}</h3>${relationList(row.matchups)}</article><article class="panel"><h3>${h(tr("synergy"))}</h3>${relationList(row.synergies)}</article><article class="panel"><h3>${h(tr("items"))}</h3>${itemList(row.top_items)}</article></div>`;
}

function championRoleStats(profile) {
  if (!profile) return `<article class="panel champion-role-stats"><h3>${h(tr("roleStatistics"))}</h3><p class="muted">${h(tr("noData"))}</p></article>`;
  const patches = (profile.used_patches || []).join(" → ") || "—";
  return `<article class="panel champion-role-stats"><div class="role-stats-heading"><div><h3>${h(tr("roleStatistics"))}</h3><small>${h(tr("rolePatches"))}: ${h(patches)}</small></div><span class="eligibility ${profile.sufficient ? "yes" : "no"}">${h(tr(profile.sufficient ? "roleSampleEnough" : "roleSampleLow"))} · ${profile.total_matches}/${profile.required_matches}</span></div><div class="table-scroll"><table><thead><tr><th>${h(tr("role"))}</th><th>${h(tr("games"))}</th><th>${h(tr("roleShare"))}</th><th>${h(tr("winRate"))}</th></tr></thead><tbody>${(profile.roles || []).map((role) => `<tr><td><strong>${h(label(role.role))}</strong></td><td>${number(role.matches)}</td><td>${pct(role.share)}</td><td>${pct(role.win_rate)}</td></tr>`).join("")}</tbody></table></div></article>`;
}

function metric(title, value) { return `<article class="metric"><small>${h(title)}</small><strong>${h(value)}</strong></article>`; }
function patchHistory(row) {
  const rows = row.patch_history || [];
  const selected = (state.preview ? state.profile : state.live?.active_profile)?.patch;
  return `<article class="panel patch-history"><div class="patch-heading"><h3>${h(tr("patchHistory"))}</h3><span class="patch-chip">${h(selected ? label(selected) : tr("all"))}</span></div>${rows.length ? `<div class="table-scroll"><table><thead><tr><th>${h(tr("patch"))}</th><th>${h(tr("sampleCount"))}</th><th>${h(tr("winRate"))}</th><th>${h(tr("pickRate"))}</th><th>${h(tr("banRate"))}</th><th>${h(tr("bans"))}</th></tr></thead><tbody>${rows.map((patch) => `<tr><td><strong>${h(patch.patch)}</strong></td><td>${patch.pick_count}</td><td>${pct(patch.win_rate)}</td><td>${pct(patch.pick_rate)}</td><td>${pct(patch.ban_rate)}</td><td>${patch.ban_count}</td></tr>`).join("")}</tbody></table></div>` : `<p class="muted">${h(tr("noData"))}</p>`}</article>`;
}
function components(row) { const values = row.score?.components; return !values ? "" : `<article class="panel score-components"><h3>${h(tr("score"))}</h3>${Object.entries(values).map(([key, value]) => `<div><span>${h(key)}</span><progress max="100" value="${Number(value)}"></progress><strong>${Number(value).toFixed(1)}</strong></div>`).join("")}</article>`; }
function statList(stats = {}) { return `<dl class="stat-list">${Object.entries(stats).map(([key, value]) => `<div><dt>${h(key.replaceAll("_", " "))}</dt><dd>${h(value)}</dd></div>`).join("")}</dl>`; }
function relationList(rows = []) {
  const activeIds = new Set((state.live?.champion_info || []).map((row) => row.champion_id));
  const visibleRows = rows.filter((row) => activeIds.has(row.champion_id)).slice(0, 12);
  return visibleRows.length ? `<ul class="relation-list">${visibleRows.map((row) => `<li><strong>${h(championName(row.champion_id))}</strong><span>${row.games} ${h(tr("games"))} · ${pct(row.win_rate)}</span></li>`).join("")}</ul>` : `<p class="muted">${h(tr("noData"))}</p>`;
}
function itemList(rows = []) { return rows.length ? `<ul class="relation-list">${rows.slice(0, 12).map((row) => `<li><strong>#${h(row.item_id)}</strong><span>${row.games} ${h(tr("games"))} · ${pct(row.adoption_rate)}</span></li>`).join("")}</ul>` : `<p class="muted">${h(tr("noData"))}</p>`; }

function renderDraft(content) {
  const draft = state.draft;
  if (!draft) { content.innerHTML = `<div class="empty">${h(tr("mockDraftLoading"))}</div>`; return; }
  const set = (draft.sets || []).find((candidate) => Number(candidate.set_number) === Number(draft.current_set))
    || DraftForm.currentSet(draft);
  const playerTeam = (draft.team_options || []).find((team) => team.player_team);
  const opponent = (draft.team_options || []).find((team) => team.team_id === draft.opponent_team_id);
  const recommendation = draft.recommendation;
  const ourSide = DraftForm.currentSide(draft);
  const blueTeam = ourSide === "blue" ? playerTeam : opponent;
  const redTeam = ourSide === "red" ? playerTeam : opponent;
  content.innerHTML = `<section class="panel draft-control-bar"><label><span>${h(tr("draftOpponent"))}</span><input id="draftOpponentSearch" class="draft-opponent-search" list="draftOpponentOptions" value="${h(opponent?.team_name || "")}" placeholder="${h(tr("selectOpponent"))}"><datalist id="draftOpponentOptions">${(draft.team_options || []).filter((team) => !team.player_team).map((team) => `<option value="${h(team.team_name)}" data-team-id="${team.team_id}"></option>`).join("")}</datalist></label><label><span>${h(tr("draftRule"))}</span><select id="draftRule" class="draft-rule-select"><option value="classic" ${draft.rule === "classic" ? "selected" : ""}>${h(label("classic"))}</option><option value="fearless" ${draft.rule === "fearless" ? "selected" : ""}>${h(label("fearless"))}</option><option value="fearless_hard" ${draft.rule === "fearless_hard" ? "selected" : ""}>${h(label("fearless_hard"))}</option></select></label><label><span>${h(tr("ourSide"))}</span><select id="draftSide" class="draft-side-select"><option value="blue" ${ourSide === "blue" ? "selected" : ""}>${h(tr("blue"))}</option><option value="red" ${ourSide === "red" ? "selected" : ""}>${h(tr("red"))}</option></select></label><div class="draft-set-tabs">${[1, 2, 3, 4, 5].map((setNumber) => `<button data-draft-set="${setNumber}" class="${draft.current_set === setNumber ? "active" : ""}">${setNumber}${h(tr("setSuffix"))}</button>`).join("")}</div></section>
    ${draftEvaluationPanel(draft.evaluation)}
    <section class="draft-mode-bar">${[["our_ban", "ourBan"], ["our_pick", "ourPick"], ["enemy_ban", "enemyBan"], ["enemy_pick", "enemyPick"]].map(([mode, key]) => `<button class="draft-mode-button ${state.draftMode === mode ? "active" : ""}" data-draft-mode="${mode}">${h(tr(key))}</button>`).join("")}<button id="draftUndo" class="secondary">${h(tr("undo"))}</button><button id="draftReset" class="danger">${h(tr("resetSet"))}</button></section>
    <section class="mock-draft-board">
      ${draftSidePanel("blue", blueTeam, ourSide === "blue", set.blue_bans, set.blue_picks, draft.blue_gate, draft.excluded_blue)}
      <div class="draft-champion-pool"><div class="draft-pool-tools"><label class="search-wrap"><span>⌕</span><input id="draftChampionSearch" value="${h(state.draftSearch)}" placeholder="${h(tr("searchChampion"))}"></label><select id="draftRoleFilter"><option value="all">${h(tr("all"))}</option>${["top", "jungle", "mid", "bot", "support"].map((role) => `<option value="${role}" ${state.draftRole === role ? "selected" : ""}>${h(label(role))}</option>`).join("")}</select></div><div id="draftChampionGrid" class="draft-champion-grid"></div></div>
      ${draftSidePanel("red", redTeam, ourSide === "red", set.red_bans, set.red_picks, draft.red_gate, draft.excluded_red)}
    </section>
    ${recommendation ? `<div class="draft-projections"><article class="panel"><h3>${h(tr("projectedAlly"))}</h3>${draftChampionChips(recommendation.projected_ally)}<p>${h(tr("remainingRoles"))}: ${recommendation.ally_remaining_roles.map(label).map(h).join(" · ") || "—"}</p>${compositionBars(recommendation.ally_composition)}</article><article class="panel"><h3>${h(tr("projectedEnemy"))}</h3>${draftChampionChips(recommendation.projected_enemy)}<p>${h(tr("remainingRoles"))}: ${recommendation.enemy_remaining_roles.map(label).map(h).join(" · ") || "—"}</p>${compositionBars(recommendation.enemy_composition)}</article></div>
      <section class="draft-recommendations"><h2>${h(tr("draftTopFive"))}</h2>${recommendation.candidates.map(draftRecommendationCard).join("") || `<div class="empty">${h(tr("noData"))}</div>`}${(recommendation.notes || []).map((note) => `<p class="draft-note">${h(tr(note))}</p>`).join("")}</section>` : `<div class="empty">${h(tr("draftWaiting"))}</div>`}
    ${draftSettingsPanel(draft.settings)}`;
  renderDraftChampionPool();
  bindDraftControls();
}

function draftSidePanel(side, team, ours, bans, picks, gate, excluded = []) {
  const slots = (phase, values, capacity) => `<div class="draft-slot-list">${values.map((championId) => `<div class="draft-slot filled"><span class="champion-avatar">${h(championName(championId).slice(0, 2).toUpperCase())}</span><strong>${h(championName(championId))}</strong><button data-remove-side="${side}" data-remove-phase="${phase}" data-remove-champion="${h(championId)}" title="${h(tr("remove"))}">×</button></div>`).join("")}${Array.from({ length: Math.max(0, capacity - values.length) }, () => `<div class="draft-slot empty"><span>+</span></div>`).join("")}</div>`;
  const definite = gate?.feasible ? (gate.definitely_filled || []).map(label).join(" · ") : tr("roleConflict");
  return `<article class="draft-side-panel ${side}"><header><span class="side-badge">${h(tr(side))}</span><div><h2>${h(team?.team_name || tr("teamNotSelected"))}</h2><small>${h(tr(ours ? "ourTeam" : "enemyTeam"))}</small></div></header><section><h3>${h(tr("bans"))} · ${bans.length}/3</h3>${slots("ban", bans, 3)}</section><section><h3>${h(tr("picks"))} · ${picks.length}/5</h3>${slots("pick", picks, 5)}</section><footer><strong>${h(tr("definitelyFilled"))}</strong><span>${h(definite || tr("none"))}</span>${excluded.length ? `<strong>${h(tr("fearlessExcluded"))}</strong><span>${excluded.map(championName).map(h).join(" · ")}</span>` : ""}</footer></article>`;
}

function draftEvaluationPanel(evaluation = {}) {
  const side = (key) => {
    const value = evaluation[key];
    if (!value) return `<article class="${key}"><small>${h(tr(key))}</small><strong>--</strong></article>`;
    return `<article class="${key}"><small>${h(tr(key))}</small><strong>${Number(value.total).toFixed(1)}</strong><div><span>${h(tr("opScore"))} ${Number(value.op).toFixed(1)}</span><span>${h(tr("synergy"))} ${Number(value.synergy).toFixed(1)}</span><span>${h(tr("matchup"))} ${Number(value.matchup).toFixed(1)}</span><span>${h(tr("dataCoverage"))} ${pct(value.data_coverage)}</span></div></article>`;
  };
  return `<section class="draft-evaluation"><h2>${h(tr("currentDraftEvaluation"))}</h2><div>${side("blue")}<span class="versus">VS</span>${side("red")}</div></section>`;
}

function draftChampionChips(rows = []) {
  return `<div class="draft-chips">${rows.map((championId) => `<span title="${h(championId)}">${h(championName(championId))}</span>`).join("") || "—"}</div>`;
}

function compositionBars(value = {}) {
  return `<div class="composition-bars">${[["AD", value.ad], ["AP", value.ap], ["Tank", value.tank], ["Utility", value.utility], ["CC", value.cc]].map(([key, score]) => `<div><span>${key}</span><progress max="5" value="${Number(score) || 0}"></progress><strong>${number(score)}</strong></div>`).join("")}</div>`;
}

function draftRecommendationCard(candidate, index) {
  const components = Object.entries(candidate.components || {}).filter(([, value]) => value != null);
  const positions = gamePositionText(candidate.champion_id);
  return `<article class="draft-candidate"><span class="draft-rank">${index + 1}</span><div class="draft-candidate-main"><strong>${h(championName(candidate.champion_id))}</strong><small title="${h(candidate.champion_id)}">${h(positions)}${candidate.low_confidence ? ` · ${h(tr("confidenceLow"))}` : ""}</small></div><strong class="draft-total">${Number(candidate.total).toFixed(1)}</strong><div class="draft-components">${components.map(([key, value]) => `<span><small>${h(tr(`draftComponent_${key}`))}</small><b>${Number(value).toFixed(1)}</b></span>`).join("")}</div></article>`;
}

function draftSettingsPanel(settings) {
  const slider = (key, labelKey, toggle = null) => `<label class="draft-weight ${toggle && !settings[toggle] ? "disabled" : ""}">${toggle ? `<input class="draft-weight-toggle" data-toggle="${toggle}" type="checkbox" ${settings[toggle] ? "checked" : ""}>` : ""}<span>${h(tr(labelKey))}</span><input data-weight="${key}" type="range" min="0" max="100" value="${settings[key]}"><output>${settings[key]}</output></label>`;
  return `<article class="panel draft-settings"><h3>${h(tr("draftSettings"))}</h3><div class="two-column"><section><h4>${h(tr("pickSettings"))}</h4>${slider("pick_op", "opScore")}${slider("pick_matchup", "matchup")}${slider("pick_synergy", "synergy")}${slider("pick_composition", "composition")}${slider("pick_denial", "denial")}<label class="editor-toggle-row"><span>${h(tr("roleGate"))}</span><input id="pickRoleGate" type="checkbox" ${settings.pick_role_gate ? "checked" : ""}></label></section><section><h4>${h(tr("banSettings"))}</h4>${slider("ban_preference", "opponentPreference")}${slider("ban_op", "opScore")}${slider("ban_threat", "banThreat", "ban_threat_enabled")}${slider("ban_synergy", "banProjectedSynergy", "ban_synergy_enabled")}${slider("ban_composition", "banProjectedComposition", "ban_composition_enabled")}<label class="editor-toggle-row"><span>${h(tr("roleGate"))}</span><input id="banRoleGate" type="checkbox" ${settings.ban_role_gate ? "checked" : ""}></label></section></div><div class="profile-actions"><button id="draftDefaults" class="secondary">${h(tr("restoreDefaults"))}</button><button id="draftSaveSettings" class="primary">${h(tr("saveSettings"))}</button></div></article>`;
}

function draftSettingsFromForm(defaults = null) {
  if (defaults) return defaults;
  const current = structuredClone(state.draft.settings);
  document.querySelectorAll("[data-weight]").forEach((input) => { current[input.dataset.weight] = Number(input.value); });
  document.querySelectorAll("[data-toggle]").forEach((input) => { current[input.dataset.toggle] = input.checked; });
  current.pick_role_gate = $("#pickRoleGate").checked;
  current.ban_role_gate = $("#banRoleGate").checked;
  return current;
}

function renderDraftChampionPool() {
  const container = $("#draftChampionGrid");
  if (!container || !state.draft) return;
  const query = state.draftSearch.trim().toLowerCase();
  const available = new Set(state.draft.available_champions || []);
  const rows = champions().filter((row) => available.has(row.champion_id))
    .filter((row) => !query || row.display_name.toLowerCase().includes(query) || row.champion_id.toLowerCase().includes(query))
    .filter((row) => state.draftRole === "all" || gamePositions(row.champion_id).includes(state.draftRole));
  container.innerHTML = rows.map((row) => {
    const availability = DraftForm.championAvailability(state.draft, state.draftMode, row.champion_id);
    const reason = availability.reason ? tr(`draftUnavailable_${availability.reason}`) : "";
    return `<button class="draft-champion-button" data-draft-champion="${h(row.champion_id)}" ${availability.disabled ? "disabled" : ""} title="${h(reason || row.champion_id)}"><span class="champion-avatar">${h(row.display_name.slice(0, 2).toUpperCase())}</span><strong>${h(row.display_name)}</strong><small>${h(gamePositionText(row.champion_id))} · ${tierLabel(row.tier)}</small></button>`;
  }).join("") || `<div class="empty">${h(tr("noData"))}</div>`;
  container.querySelectorAll("[data-draft-champion]").forEach((button) => button.addEventListener("click", async () => {
    const target = DraftForm.modeTarget(state.draftMode, DraftForm.currentSide(state.draft));
    try {
      state.draft = await window.tfm2.request("APPLY_MOCK_DRAFT_ACTION", { ...target, champion_id: button.dataset.draftChampion });
      renderDraft($("#content"));
    } catch (error) { toast(bridgeErrorText(error), true); }
  }));
}

function bindDraftControls() {
  document.querySelectorAll("[data-weight]").forEach((input) => { input.oninput = () => { input.nextElementSibling.value = input.value; }; });
  const setContext = async (payload) => { try { state.draft = await window.tfm2.request("SET_MOCK_DRAFT_CONTEXT", payload); renderDraft($("#content")); } catch (error) { toast(bridgeErrorText(error), true); } };
  $("#draftOpponentSearch").addEventListener("change", (event) => {
    const team = (state.draft.team_options || []).find((row) => !row.player_team && (row.team_name === event.target.value || String(row.team_id) === event.target.value));
    if (team) setContext({ opponent_team_id: team.team_id });
  });
  $("#draftRule").addEventListener("change", (event) => setContext({ rule: event.target.value }));
  $("#draftSide").addEventListener("change", (event) => setContext({ player_side: event.target.value }));
  document.querySelectorAll("[data-draft-set]").forEach((button) => button.addEventListener("click", () => setContext({ current_set: Number(button.dataset.draftSet) })));
  document.querySelectorAll("[data-draft-mode]").forEach((button) => button.addEventListener("click", () => {
    state.draftMode = button.dataset.draftMode;
    const target = DraftForm.modeTarget(state.draftMode, DraftForm.currentSide(state.draft));
    if (state.draftMode.startsWith("our_")) setContext({ recommendation_phase: target.phase });
    else renderDraft($("#content"));
  }));
  document.querySelectorAll("[data-remove-champion]").forEach((button) => button.addEventListener("click", async () => { try { state.draft = await window.tfm2.request("REMOVE_MOCK_DRAFT_ACTION", { side: button.dataset.removeSide, phase: button.dataset.removePhase, champion_id: button.dataset.removeChampion }); renderDraft($("#content")); } catch (error) { toast(bridgeErrorText(error), true); } }));
  $("#draftUndo").onclick = async () => { try { state.draft = await window.tfm2.request("UNDO_MOCK_DRAFT"); renderDraft($("#content")); } catch (error) { toast(bridgeErrorText(error), true); } };
  $("#draftReset").onclick = async () => { try { state.draft = await window.tfm2.request("RESET_MOCK_DRAFT_SET"); renderDraft($("#content")); } catch (error) { toast(bridgeErrorText(error), true); } };
  $("#draftChampionSearch").addEventListener("input", (event) => { state.draftSearch = event.target.value; renderDraftChampionPool(); });
  $("#draftRoleFilter").addEventListener("change", (event) => { state.draftRole = event.target.value; renderDraftChampionPool(); });
  $("#draftSaveSettings").onclick = async () => { try { state.draft = await window.tfm2.request("SET_DRAFT_SETTINGS", draftSettingsFromForm()); render(); } catch (error) { toast(bridgeErrorText(error), true); } };
  $("#draftDefaults").onclick = async () => { try { state.draft = await window.tfm2.request("SET_DRAFT_SETTINGS", { pick_op: 40, pick_matchup: 20, pick_synergy: 20, pick_composition: 10, pick_denial: 10, pick_role_gate: true, ban_preference: 40, ban_op: 60, ban_threat_enabled: false, ban_threat: 10, ban_synergy_enabled: false, ban_synergy: 10, ban_composition_enabled: false, ban_composition: 10, ban_role_gate: false }); render(); } catch (error) { toast(bridgeErrorText(error), true); } };
}

function renderAbout(content) {
  content.innerHTML = `<section class="about-card"><span class="brand-mark large">A</span><h2>TFM2 Atlas Dashboard 1.0.33</h2><p>TFM2 · 0.5.5</p><div class="credits"><strong>${h(tr("author"))}</strong><span>${h(tr("creditsDashboard"))}</span></div><p class="privacy">${h(tr("privacy"))}</p></section>`;
}

$("#search").addEventListener("input", (event) => { state.search = event.target.value; renderChampionSidebar(); });
$("#sidebarToggle").addEventListener("click", () => setSidebarHidden(true));
$("#sidebarReveal").addEventListener("click", () => setSidebarHidden(false));
document.querySelectorAll("[data-language]").forEach((button) => button.addEventListener("click", async () => { state.language = button.dataset.language; await window.tfm2.setLanguage(state.language); render(); }));
document.addEventListener("click", (event) => { if (event.target?.id === "retry") refresh(); });

(async () => {
  const settings = await window.tfm2.settings();
  state.language = settings.language;
  state.messages = settings.messages;
  state.profile = settings.lastProfile || defaultProfile();
  state.initializedProfile = Boolean(settings.lastProfile);
  window.tfm2.onStateChanged((message) => {
    if (document.visibilityState === "hidden") return;
    const plan = RefreshPlan.fromScopes(message?.scopes, state.page);
    if (!plan.relevant) return;
    refresh(plan);
  });
  await refresh();
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "visible") refresh();
  });
})();
