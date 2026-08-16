const PLAYER_STAT_KEYS = ["last_hit", "skill_avoid", "skill_hit", "control_speed", "positioning", "judgement", "mental", "concentration", "order", "roaming", "aggressive", "ego"];
const STAFF_STAT_KEYS = ["banpick", "strategy", "negotiation", "judge_ability", "judge_potential", "feedback", "power_analysis", "control_coaching", "judgment_coaching", "mental_coaching"];
const POSITION_KEYS = ["top", "jungle", "mid", "bottom", "support"];
const REGIONS = [[0, "KR"], [1, "CN"], [2, "EU"], [3, "NA"], [4, "SA"], [5, "JP"]];
const statLabels = {
  ko: { last_hit: "막타", skill_avoid: "스킬 회피", skill_hit: "스킬 적중", control_speed: "조작 속도", positioning: "포지셔닝", judgement: "판단", mental: "멘탈", concentration: "집중력", order: "오더", roaming: "로밍", aggressive: "공격성", ego: "에고", banpick: "밴픽", strategy: "전략", negotiation: "협상", judge_ability: "능력 판단", judge_potential: "잠재력 판단", feedback: "피드백", power_analysis: "전력 분석", control_coaching: "조작 코칭", judgment_coaching: "판단 코칭", mental_coaching: "멘탈 코칭" },
  en: { last_hit: "Last Hit", skill_avoid: "Skill Avoidance", skill_hit: "Skill Accuracy", control_speed: "Control Speed", positioning: "Positioning", judgement: "Judgement", mental: "Mental", concentration: "Concentration", order: "Shotcalling", roaming: "Roaming", aggressive: "Aggression", ego: "Ego", banpick: "Drafting", strategy: "Strategy", negotiation: "Negotiation", judge_ability: "Ability Evaluation", judge_potential: "Potential Evaluation", feedback: "Feedback", power_analysis: "Power Analysis", control_coaching: "Control Coaching", judgment_coaching: "Judgement Coaching", mental_coaching: "Mental Coaching" },
};

const text = {
  ko: {
    hideMenu: "왼쪽 메뉴 숨기기", showMenu: "왼쪽 메뉴 열기", searchEditor: "현재 목록 검색", connected: "커리어 연결됨", disconnected: "게임 연결 대기", coreStatus: "Core", editorStatus: "Editor", online: "연결됨", offline: "대기", about: "정보",
    player: "선수 편집", staff: "스태프 편집", recruitment: "영입 설정", economy: "재정", locks: "잠금 관리자", name: "이름", team: "팀", position: "포지션",
    apply: "게임에 적용", refresh: "새로고침", maxAll: "모두 100", playerStats: "선수 능력치", staffStats: "스태프 능력치", salary: "연봉", potential: "잠재력",
    mastery: "챔피언 숙련도", activeChampions: "현재 패치 활성 챔피언", inactiveChampions: "현재 패치 비활성 챔피언", saveMastery: "숙련도 적용", movePlayer: "선수 이동",
    destination: "이동할 팀", moveNow: "즉시 이동", transferSuccess: "영입 항상 성공", instantRetry: "영입 즉시 재시도", totalBalance: "보유 자금", transferBudget: "이적 예산",
    salaryBudget: "연봉 예산", applyEconomy: "재정 적용", noSelection: "왼쪽 목록에서 대상을 선택하세요.", noData: "표시할 데이터가 없습니다.", applied: "게임 서버와 현재 화면에 적용했습니다.",
    moved: "서버 적용 완료 · 게임 화면 동기화 중", moveVerificationFailed: "이동 후 재조회 결과가 대상 팀과 일치하지 않습니다.", bridgeRequired: "tfm2_atlas_core, tfm2_atlas_client_055, tfm2_atlas_editor 모드를 모두 켜고 커리어를 시작하세요.",
    target: "대상", group: "종류", values: "값", unlock: "해제", loading: "불러오는 중…", editor: "Editor", condition: "컨디션", stress: "스트레스", freeAgent: "자유계약",
    age: "나이", communication: "의사소통", positions: "포지션 숙련도", playerContract: "선수 계약", staffContract: "스태프 계약", contractStart: "계약 시작", contractEnd: "계약 종료",
    transferFee: "이적료", squadStatus: "선수단 지위", core: "핵심 선수", important: "중요 선수", general: "일반 선수", sub: "교체 선수", prospect: "유망주", incentives: "계약 보너스",
    pogBonus: "POM 보너스", leagueBonus: "리그 순위 보너스", leagueRank: "리그 순위", matchBonus: "출장 보너스", winBonus: "승리 보너스", applyContract: "계약 적용",
    lockPlayer: "선수 전체 고정", lockStaff: "스태프 전체 고정", locked: "현재 값 전체를 고정했습니다.", nativeRegion: "주 지역", tooManyPositions: "활성 포지션은 최대 3개입니다.", teamFilter: "팀 선택", allTeams: "전체 팀", author: "Author: ehcho", creditsEditor: "Inspired by TFM2 Editor by jal-io", privacy: "편집 요청은 현재 커리어에 직접 적용되며 별도 세이브 파일을 요구하지 않습니다.",
  },
  en: {
    hideMenu: "Hide left menu", showMenu: "Show left menu", searchEditor: "Search current list", connected: "Career connected", disconnected: "Waiting for game", coreStatus: "Core", editorStatus: "Editor", online: "Connected", offline: "Waiting", about: "About",
    player: "Player Editor", staff: "Staff Editor", recruitment: "Recruitment", economy: "Economy", locks: "Lock Manager", name: "Name", team: "Team", position: "Position",
    apply: "Apply in game", refresh: "Refresh", maxAll: "Set all to 100", playerStats: "Player Stats", staffStats: "Staff Stats", salary: "Annual salary", potential: "Potential",
    mastery: "Champion Mastery", activeChampions: "Active champions in current patch", inactiveChampions: "Inactive champions in current patch", saveMastery: "Apply mastery", movePlayer: "Move Player",
    destination: "Destination team", moveNow: "Move now", transferSuccess: "Always accept recruitment", instantRetry: "Instant recruitment retry", totalBalance: "Total balance", transferBudget: "Transfer budget",
    salaryBudget: "Salary budget", applyEconomy: "Apply economy", noSelection: "Select a target from the left list.", noData: "No data to show.", applied: "Applied to the game server and current screen.",
    moved: "Server applied · game screen synchronizing", moveVerificationFailed: "Readback team did not match the destination.", bridgeRequired: "Enable tfm2_atlas_core, tfm2_atlas_client_055 and tfm2_atlas_editor, then start a career.",
    target: "Target", group: "Group", values: "Values", unlock: "Unlock", loading: "Loading…", editor: "Editor", condition: "Condition", stress: "Stress", freeAgent: "Free Agent",
    age: "Age", communication: "Communication", positions: "Position Proficiency", playerContract: "Player Contract", staffContract: "Staff Contract", contractStart: "Contract start", contractEnd: "Contract end",
    transferFee: "Transfer fee", squadStatus: "Squad status", core: "Core", important: "Important", general: "General", sub: "Substitute", prospect: "Prospect", incentives: "Contract bonuses",
    pogBonus: "POM bonus", leagueBonus: "League-rank bonus", leagueRank: "League rank", matchBonus: "Appearance bonus", winBonus: "Win bonus", applyContract: "Apply contract",
    lockPlayer: "Lock entire player", lockStaff: "Lock entire staff", locked: "Locked every current value.", nativeRegion: "Primary region", tooManyPositions: "At most three positions can be active.", teamFilter: "Team", allTeams: "All teams", author: "Author: ehcho", creditsEditor: "Inspired by TFM2 Editor by jal-io", privacy: "Edits are applied directly to the active career and do not require a separate save file.",
  },
};

const state = {
  language: "ko", page: "player", connected: false, coreConnected: false, players: [], staffs: [], teams: [], locks: [],
  selectedPlayerId: null, selectedStaffId: null, player: null, staff: null, search: "", teamFilter: "all", loading: false,
  sidebarHidden: localStorage.getItem("tfm2.editor.sidebarHidden") === "1",
};

const $ = (selector) => document.querySelector(selector);
const h = (value) => String(value ?? "").replace(/[&<>"']/g, (character) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[character]));
const tr = (key) => text[state.language]?.[key] || text.ko[key] || key;
const clampInt = (value, min = 0, max = 100) => Math.max(min, Math.min(max, Math.trunc(Number(value))));

function toast(message, error = false) {
  const node = $("#toast"); node.textContent = message; node.classList.toggle("error", error); node.hidden = false;
  clearTimeout(toast.timer); toast.timer = setTimeout(() => { node.hidden = true; }, 3800);
}

function parseMoney(value) {
  const number = Number(String(value ?? "").replace(/[\s,$₩]/g, ""));
  if (!Number.isFinite(number) || number < 0) throw new Error("invalid_money");
  return number;
}

function formatMoney(value) {
  try { return parseMoney(value).toLocaleString(state.language === "ko" ? "ko-KR" : "en-US", { maximumFractionDigits: 2 }); }
  catch { return String(value ?? ""); }
}

function normalizeMap(value) {
  return Object.fromEntries(Object.entries(value || {}).map(([key, entry]) => [key, clampInt(entry)]));
}

function normalizePlayer(value) {
  return {
    ...value,
    id: Number(value.id),
    age: Number(value.age),
    stats: (value.stats || []).map(Number),
    positions: (value.positions || []).map(Number),
    potential: Number(value.potential),
    teamId: value.teamId == null ? null : Number(value.teamId),
    stamina: Number(value.stamina),
    condition: Number(value.condition),
    stress: Number(value.stress),
    squadStatus: value.squadStatus || "General",
    bonuses: { pog: "", league: "", rank: "1", match: "", win: "", ...(value.bonuses || {}) },
    communication: normalizeMap(value.communication),
    communicationXp: normalizeMap(value.communicationXp),
  };
}

function normalizeStaff(value) {
  return {
    ...value,
    id: Number(value.id),
    age: Number(value.age),
    stats: (value.stats || []).map(Number),
    teamId: value.teamId == null ? null : Number(value.teamId),
    communication: normalizeMap(value.communication),
  };
}

const requestOverview = EditorModel.singleFlight(
  () => window.tfm2.request("GET_EDITOR_DATA", { view: "overview" }),
);
const requestCatalog = EditorModel.singleFlight(
  () => window.tfm2.request("GET_CATALOG"),
);

async function loadLists({ silent = false } = {}) {
  if (!silent) { state.loading = true; render(); }
  try {
    const [overviewResult, catalogResult] = await Promise.allSettled([requestOverview(), requestCatalog()]);
    state.connected = overviewResult.status === "fulfilled";
    state.coreConnected = catalogResult.status === "fulfilled";
    if (!state.connected) throw overviewResult.reason;
    const overview = overviewResult.value;
    state.players = overview.players || []; state.staffs = overview.staffs || []; state.teams = overview.teams || []; state.connected = true;
    if (state.selectedPlayerId && !state.players.some((row) => row.id === state.selectedPlayerId)) state.selectedPlayerId = null;
    if (state.selectedStaffId && !state.staffs.some((row) => row.id === state.selectedStaffId)) state.selectedStaffId = null;
    if (silent) { renderConnection(); renderEntityList(); }
  } catch (error) {
    if (!silent) { state.connected = false; state.players = []; state.staffs = []; state.teams = []; }
  } finally {
    if (!silent) { state.loading = false; render(); }
  }
}

async function selectPlayer(id) { state.selectedPlayerId = Number(id); state.player = normalizePlayer(await window.tfm2.request("GET_EDITOR_DATA", { view: "player", id: Number(id) })); render(); }
async function selectStaff(id) { state.selectedStaffId = Number(id); state.staff = normalizeStaff(await window.tfm2.request("GET_EDITOR_DATA", { view: "staff", id: Number(id) })); render(); }

function applyStatic() {
  document.documentElement.lang = state.language; document.body.classList.toggle("sidebar-hidden", state.sidebarHidden);
  $("#sidebar").setAttribute("aria-hidden", String(state.sidebarHidden)); $("#sidebarReveal").hidden = !state.sidebarHidden;
  document.querySelectorAll("[data-placeholder]").forEach((node) => { node.placeholder = tr(node.dataset.placeholder); });
  document.querySelectorAll("[data-i18n-title]").forEach((node) => { node.title = tr(node.dataset.i18nTitle); node.setAttribute("aria-label", node.title); });
}

function render() { applyStatic(); renderNav(); renderConnection(); renderEntityList(); renderContent(); }

function renderNav() {
  const pages = [["player", "player", "♟"], ["staff", "staff", "♜"], ["recruitment", "recruitment", "⇄"], ["economy", "economy", "₩"], ["locks", "locks", "⌁"], ["about", "about", "ⓘ"]];
  $("#primaryNav").innerHTML = pages.map(([page, key, icon]) => `<button class="nav-item ${state.page === page ? "active" : ""}" data-page="${page}"><span>${icon}</span>${h(tr(key))}</button>`).join("");
  $("#primaryNav").querySelectorAll("[data-page]").forEach((button) => button.onclick = () => { state.page = button.dataset.page; render(); if (state.page === "locks") loadLocks(); });
  $("#pageTitle").textContent = tr(state.page);
}

function renderConnection() {
  const ready = state.connected && state.coreConnected;
  const node = $("#connection"); node.classList.toggle("online", ready);
  node.querySelector("strong").textContent = tr(ready ? "connected" : "disconnected");
  node.querySelector("small").textContent = `${tr("coreStatus")} ${tr(state.coreConnected ? "online" : "offline")} · ${tr("editorStatus")} ${tr(state.connected ? "online" : "offline")}`;
}

function entities() { return state.page === "staff" ? state.staffs : state.page === "player" || state.page === "recruitment" ? state.players : []; }
function renderEntityList() {
  const hasEntities = ["player", "staff", "recruitment"].includes(state.page);
  $("#entitySearchWrap").hidden = !hasEntities;
  $("#teamFilter").closest("label").hidden = !hasEntities;
  $("#entityList").hidden = !hasEntities;
  if (!hasEntities) return;
  const teams = EditorModel.orderedTeams(state.teams);
  const filter = $("#teamFilter");
  filter.setAttribute("aria-label", tr("teamFilter"));
  filter.title = tr("teamFilter");
  filter.innerHTML = `<option value="all">${h(tr("allTeams"))}</option>${teams.map((team) => `<option value="${team.id}" ${String(state.teamFilter) === String(team.id) ? "selected" : ""}>${h(team.name)}${team.playerTeam ? " · ★" : ""}</option>`).join("")}`;
  filter.value = teams.some((team) => String(team.id) === String(state.teamFilter)) ? String(state.teamFilter) : "all";
  filter.onchange = () => { state.teamFilter = filter.value; renderEntityList(); };
  const rows = EditorModel.filterEntities(entities(), state.search, state.teamFilter, teams);
  $("#entityList").innerHTML = rows.map((row) => `<button class="champion-mini ${(state.page === "staff" ? state.selectedStaffId : state.selectedPlayerId) === row.id ? "active" : ""}" data-entity="${row.id}"><span class="champion-avatar">${h(row.name.slice(0, 2))}</span><span><strong>${h(row.name)}</strong><small>${h(row.team)} · ${h(row.position || row.role || "")}</small></span></button>`).join("");
  $("#entityList").querySelectorAll("[data-entity]").forEach((button) => button.onclick = async () => { try { if (state.page === "staff") await selectStaff(button.dataset.entity); else await selectPlayer(button.dataset.entity); } catch (error) { toast(error.message, true); } });
}

function field(id, label, value, type = "text", attrs = "") { return `<label class="field"><span>${h(label)}</span><input id="${id}" type="${type}" value="${h(value)}" ${attrs}></label>`; }
function moneyField(id, label, value) { return field(id, label, formatMoney(value), "text", 'inputmode="decimal" data-money'); }
function statGrid(keys, values, prefix) { return `<div class="editor-stat-grid">${keys.map((key, index) => field(`${prefix}${index}`, statLabels[state.language]?.[key] || key, values[index] ?? 0, "number", 'min="0" max="100"')).join("")}</div>`; }
function selectField(id, label, options, selected) { return `<label class="field"><span>${h(label)}</span><select id="${id}">${options.map(([value, textValue]) => `<option value="${h(value)}" ${String(value) === String(selected) ? "selected" : ""}>${h(textValue)}</option>`).join("")}</select></label>`; }
function teamField(id, label, selected) { return selectField(id, label, EditorModel.orderedTeams(state.teams).map((team) => [team.id, `${team.name} · ${team.roster}${team.playerTeam ? " · ★" : ""}`]), selected); }
function communicationGrid(values, prefix, primaryRegion = null) {
  return `<div class="editor-stat-grid">${REGIONS.map(([id, label]) => field(`${prefix}${id}`, `${label}${String(primaryRegion) === String(id) ? ` · ${tr("nativeRegion")}` : ""}`, values[id] ?? 0, "number", 'min="0" max="100"')).join("")}</div>`;
}

function bindMoneyInputs(root = document) {
  root.querySelectorAll("[data-money]").forEach((input) => {
    input.onfocus = () => { try { input.value = String(parseMoney(input.value)); } catch {} };
    input.onblur = () => { input.value = formatMoney(input.value); };
  });
}

function renderContent() {
  const content = $("#content");
  if (state.loading) { content.innerHTML = `<div class="waiting"><div class="waiting-icon spinner">↻</div><h2>${h(tr("loading"))}</h2></div>`; return; }
  if (!state.connected || !state.coreConnected) { content.innerHTML = `<div class="waiting"><div class="waiting-icon">⌁</div><h2>${h(tr("disconnected"))}</h2><p>${h(tr("bridgeRequired"))}</p><button id="retry" class="primary">${h(tr("refresh"))}</button></div>`; $("#retry").onclick = () => loadLists(); return; }
  if (state.page === "player") renderPlayer(content); else if (state.page === "staff") renderStaff(content); else if (state.page === "recruitment") renderRecruitment(content); else if (state.page === "economy") renderEconomy(content); else if (state.page === "locks") renderLocks(content); else renderAbout(content);
}

function renderAbout(content) {
  content.innerHTML = `<section class="about-card"><span class="brand-mark large">A</span><h2>TFM2 Atlas Editor 1.0.33</h2><p>TFM2 · 0.5.5</p><div class="credits"><strong>${h(tr("author"))}</strong><button class="link" data-url="https://github.com/jal-io/tfm2-editor">${h(tr("creditsEditor"))}</button></div><p class="privacy">${h(tr("privacy"))}</p></section>`;
  content.querySelector("[data-url]").onclick = (event) => window.tfm2.openExternal(event.currentTarget.dataset.url);
}

function renderPlayer(content) {
  if (!state.player) { content.innerHTML = `<div class="empty">${h(tr("noSelection"))}</div>`; return; }
  const player = state.player;
  content.innerHTML = `<div class="editor-toolbar"><button id="refreshSelected" class="secondary">${h(tr("refresh"))}</button><button id="maxPlayer" class="secondary">${h(tr("maxAll"))}</button><button id="openMastery" class="secondary">${h(tr("mastery"))}</button><button id="lockPlayer" class="secondary">${h(tr("lockPlayer"))}</button><button id="applyPlayer" class="primary">${h(tr("apply"))}</button></div>
    <article class="panel editor-card"><h3>${h(player.name)} · #${player.id}</h3><div class="editor-form-grid">${field("playerName", tr("name"), player.name)}${field("playerAge", tr("age"), player.age, "number", 'min="16" max="100"')}${moneyField("playerSalary", tr("salary"), player.annualSalary)}${field("playerPotential", tr("potential"), player.potential, "number", 'min="1" max="100"')}${field("playerStress", tr("stress"), player.stress, "number", 'min="0" max="100"')}${field("playerCondition", tr("condition"), player.condition, "number", 'min="0" max="100"')}</div></article>
    <article class="panel editor-card"><h3>${h(tr("playerStats"))}</h3>${statGrid(PLAYER_STAT_KEYS, player.stats, "ps")}</article>
    <article class="panel editor-card"><h3>${h(tr("positions"))}</h3>${statGrid(POSITION_KEYS, player.positions, "pp")}</article>
    <article class="panel editor-card"><h3>${h(tr("communication"))}</h3>${communicationGrid(player.communication, "pc", player.primaryRegion)}</article>
    ${playerContractCard(player)}`;
  bindMoneyInputs(content);
  $("#refreshSelected").onclick = () => selectPlayer(player.id);
  $("#maxPlayer").onclick = () => { PLAYER_STAT_KEYS.forEach((_, index) => { $(`#ps${index}`).value = 100; }); };
  $("#applyPlayer").onclick = () => applyPlayer(); $("#openMastery").onclick = openMastery; $("#lockPlayer").onclick = lockEntirePlayer; $("#applyPlayerContract").onclick = applyPlayerContract;
}

function playerContractCard(player) {
  const bonus = (id, label, value) => `<label class="bonus-field"><input id="${id}Enabled" type="checkbox" ${value !== "" ? "checked" : ""}><span>${h(label)}</span><input id="${id}" type="text" inputmode="decimal" data-money value="${h(formatMoney(value || 0))}"></label>`;
  return `<article class="panel editor-card"><h3>${h(tr("playerContract"))}</h3><div class="editor-form-grid">${teamField("contractTeam", tr("team"), player.teamId)}${field("contractStart", tr("contractStart"), player.startDate, "date")}${field("contractEnd", tr("contractEnd"), player.endDate, "date")}${moneyField("contractSalary", tr("salary"), player.annualSalary || 0)}${moneyField("contractFee", tr("transferFee"), player.transferFee || 0)}${selectField("squadStatus", tr("squadStatus"), [["Core", tr("core")], ["Important", tr("important")], ["General", tr("general")], ["Sub", tr("sub")], ["Prospect", tr("prospect")]], player.squadStatus)}</div><h4>${h(tr("incentives"))}</h4><div class="contract-bonuses">${bonus("pogBonus", tr("pogBonus"), player.bonuses.pog)}${bonus("leagueBonus", tr("leagueBonus"), player.bonuses.league)}${field("leagueRank", tr("leagueRank"), player.bonuses.rank, "number", 'min="1" max="10"')}${bonus("matchBonus", tr("matchBonus"), player.bonuses.match)}${bonus("winBonus", tr("winBonus"), player.bonuses.win)}</div><button id="applyPlayerContract" class="primary">${h(tr("applyContract"))}</button></article>`;
}

function playerFormValues() {
  const positions = POSITION_KEYS.map((_, index) => clampInt($(`#pp${index}`).value));
  if (positions.filter((value) => value > 0).length > 3) throw new Error(tr("tooManyPositions"));
  return {
    name: $("#playerName").value.trim(), age: clampInt($("#playerAge").value, 16, 100), salary: parseMoney($("#playerSalary").value),
    potential: clampInt($("#playerPotential").value, 1, 100), stress: clampInt($("#playerStress").value), condition: clampInt($("#playerCondition").value),
    stats: PLAYER_STAT_KEYS.map((_, index) => clampInt($(`#ps${index}`).value, 1, 100)), positions,
    communication: Object.fromEntries(REGIONS.map(([id]) => [id, clampInt($(`#pc${id}`).value)])),
  };
}

async function applyPlayer({ quiet = false } = {}) {
  const id = state.player.id;
  try {
    const values = playerFormValues();
    const readback = await window.tfm2.request("APPLY_PLAYER_EDIT", {
      athlete_id: id,
      name: values.name,
      age: values.age,
      stats: Object.fromEntries(PLAYER_STAT_KEYS.map((key, index) => [key, values.stats[index]])),
      positions: Object.fromEntries(POSITION_KEYS.map((key, index) => [key, values.positions[index]])),
      potential: values.potential,
      stress: values.stress,
      condition: values.condition,
      annual_salary: state.player.teamId == null ? null : values.salary,
      communication: values.communication,
      mastery: [],
      contract: null,
    });
    state.player = normalizePlayer(await window.tfm2.request("GET_EDITOR_DATA", { view: "player", id }));
    await loadLists({ silent: true }); state.selectedPlayerId = id;
    if (state.player.name !== values.name || state.player.age !== values.age || state.player.potential !== values.potential || state.player.stats.some((value, index) => value !== values.stats[index])) throw new Error("player_readback_mismatch");
    if (!quiet) { toast(tr("applied")); render(); }
    return state.player;
  } catch (error) { if (quiet) throw error; toast(error.message, true); return null; }
}

async function applyPlayerContract() {
  const id = state.player.id;
  try {
    const enabledValue = (idValue) => $(`#${idValue}Enabled`).checked ? parseMoney($(`#${idValue}`).value) : 0;
    const incentives = [];
    if ($("#pogBonusEnabled").checked) incentives.push({ OnPog: { bonus: enabledValue("pogBonus") } });
    if ($("#leagueBonusEnabled").checked) incentives.push({ OnLeagueRank: { bonus: enabledValue("leagueBonus"), rank: clampInt($("#leagueRank").value, 1, 10) } });
    if ($("#matchBonusEnabled").checked) incentives.push({ OnMatch: { bonus: enabledValue("matchBonus") } });
    if ($("#winBonusEnabled").checked) incentives.push({ OnWin: { bonus: enabledValue("winBonus") } });
    const readback = await window.tfm2.request("APPLY_PLAYER_EDIT", { athlete_id: id, contract: { team_id: Number($("#contractTeam").value), start_date: $("#contractStart").value, end_date: $("#contractEnd").value, annual_salary: parseMoney($("#contractSalary").value), transfer_fee: parseMoney($("#contractFee").value), squad_status: $("#squadStatus").value, incentives } });
    state.player = normalizePlayer(await window.tfm2.request("GET_EDITOR_DATA", { view: "player", id })); await loadLists({ silent: true }); render(); toast(tr("applied"));
  } catch (error) { toast(error.message, true); }
}

function renderStaff(content) {
  if (!state.staff) { content.innerHTML = `<div class="empty">${h(tr("noSelection"))}</div>`; return; }
  const staff = state.staff;
  content.innerHTML = `<div class="editor-toolbar"><button id="refreshStaff" class="secondary">${h(tr("refresh"))}</button><button id="maxStaff" class="secondary">${h(tr("maxAll"))}</button><button id="lockStaff" class="secondary">${h(tr("lockStaff"))}</button><button id="applyStaff" class="primary">${h(tr("apply"))}</button></div>
    <article class="panel editor-card"><h3>${h(staff.name)} · #${staff.id}</h3><div class="editor-form-grid">${field("staffName", tr("name"), staff.name)}${field("staffAge", tr("age"), staff.age, "number", 'min="16" max="100"')}${moneyField("staffSalary", tr("salary"), staff.annualSalary)}</div></article>
    <article class="panel editor-card"><h3>${h(tr("staffStats"))}</h3>${statGrid(STAFF_STAT_KEYS, staff.stats, "ss")}</article>
    <article class="panel editor-card"><h3>${h(tr("communication"))}</h3>${communicationGrid(staff.communication, "sc")}</article>
    <article class="panel editor-card"><h3>${h(tr("staffContract"))}</h3><div class="editor-form-grid">${teamField("staffContractTeam", tr("team"), staff.teamId)}${field("staffContractStart", tr("contractStart"), staff.startDate, "date")}${field("staffContractEnd", tr("contractEnd"), staff.endDate, "date")}${moneyField("staffContractSalary", tr("salary"), staff.annualSalary || 0)}</div><button id="applyStaffContract" class="primary">${h(tr("applyContract"))}</button></article>`;
  bindMoneyInputs(content);
  $("#refreshStaff").onclick = () => selectStaff(staff.id); $("#maxStaff").onclick = () => { STAFF_STAT_KEYS.forEach((_, index) => { $(`#ss${index}`).value = 100; }); };
  $("#applyStaff").onclick = () => applyStaff(); $("#lockStaff").onclick = lockEntireStaff; $("#applyStaffContract").onclick = applyStaffContract;
}

function staffFormValues() {
  return { name: $("#staffName").value.trim(), age: clampInt($("#staffAge").value, 16, 100), salary: parseMoney($("#staffSalary").value), stats: STAFF_STAT_KEYS.map((_, index) => clampInt($(`#ss${index}`).value, 1, 100)), communication: Object.fromEntries(REGIONS.map(([id]) => [id, clampInt($(`#sc${id}`).value)])) };
}

async function applyStaff({ quiet = false } = {}) {
  const id = state.staff.id;
  try {
    const values = staffFormValues();
    const readback = await window.tfm2.request("APPLY_STAFF_EDIT", { staff_id: id, name: values.name, age: values.age, stats: Object.fromEntries(STAFF_STAT_KEYS.map((key, index) => [key, values.stats[index]])), annual_salary: state.staff.teamId == null ? null : values.salary, communication: values.communication, contract: null });
    state.staff = normalizeStaff(await window.tfm2.request("GET_EDITOR_DATA", { view: "staff", id })); await loadLists({ silent: true }); state.selectedStaffId = id;
    if (state.staff.name !== values.name || state.staff.age !== values.age || state.staff.stats.some((value, index) => value !== values.stats[index])) throw new Error("staff_readback_mismatch");
    if (!quiet) { toast(tr("applied")); render(); }
    return state.staff;
  } catch (error) { if (quiet) throw error; toast(error.message, true); return null; }
}

async function applyStaffContract() {
  const id = state.staff.id;
  try {
    const readback = await window.tfm2.request("APPLY_STAFF_EDIT", { staff_id: id, contract: { team_id: Number($("#staffContractTeam").value), start_date: $("#staffContractStart").value, end_date: $("#staffContractEnd").value, annual_salary: parseMoney($("#staffContractSalary").value) } });
    state.staff = normalizeStaff(await window.tfm2.request("GET_EDITOR_DATA", { view: "staff", id })); await loadLists({ silent: true }); render(); toast(tr("applied"));
  } catch (error) { toast(error.message, true); }
}

function renderRecruitment(content) {
  const player = state.player; const playerRow = state.players.find((row) => row.id === state.selectedPlayerId);
  content.innerHTML = `<div class="two-column"><article class="panel editor-card"><h3>${h(tr("movePlayer"))}</h3>${player ? `<p>${h(player.name)} · ${h(playerRow?.team || "")}</p>${teamField("destinationTeam", tr("destination"), player.teamId)}<button id="movePlayer" class="primary">${h(tr("moveNow"))}</button>` : `<p>${h(tr("noSelection"))}</p>`}</article><article class="panel editor-card"><h3>${h(tr("recruitment"))}</h3><label class="editor-toggle-row"><span>${h(tr("transferSuccess"))}</span><input id="transferSuccess" type="checkbox"></label><label class="editor-toggle-row"><span>${h(tr("instantRetry"))}</span><input id="instantRetry" type="checkbox"></label></article></div>`;
  if (player) $("#movePlayer").onclick = moveSelectedPlayer; loadRecruitmentSettings();
}

async function moveSelectedPlayer() {
  const id = state.selectedPlayerId; const teamId = Number($("#destinationTeam").value); const team = state.teams.find((row) => row.id === teamId);
  try {
    const readback = await window.tfm2.request("MOVE_PLAYER", { athlete_id: id, team_id: teamId });
    const player = normalizePlayer(await window.tfm2.request("GET_EDITOR_DATA", { view: "player", id })); state.selectedPlayerId = id; state.player = player;
    const row = state.players.find((candidate) => candidate.id === id); if (row) row.team = team?.name || row.team;
    if (player.teamId !== teamId) throw new Error(tr("moveVerificationFailed"));
    toast(tr("moved")); render();
  } catch (error) { toast(error.message, true); }
}

async function loadRecruitmentSettings() {
  try {
    const settings = await window.tfm2.request("GET_EDITOR_DATA", { view: "recruitment" });
    $("#transferSuccess").checked = Boolean(settings.transfer_always_success);
    $("#instantRetry").checked = Boolean(settings.instant_retry);
    const save = async () => window.tfm2.request("APPLY_EDITOR_SETTINGS", { recruitment: { transfer_always_success: $("#transferSuccess").checked, instant_retry: $("#instantRetry").checked } });
    $("#transferSuccess").onchange = async () => { try { await save(); } catch (error) { toast(error.message, true); } };
    $("#instantRetry").onchange = async () => { try { await save(); } catch (error) { toast(error.message, true); } };
  } catch (error) { toast(error.message, true); }
}

async function renderEconomy(content) {
  content.innerHTML = `<div class="waiting"><div class="waiting-icon spinner">↻</div></div>`;
  try {
    const economy = await window.tfm2.request("GET_EDITOR_DATA", { view: "economy" }); content.innerHTML = `<article class="panel editor-card"><h3>${h(tr("economy"))}</h3><div class="editor-form-grid">${moneyField("money", tr("totalBalance"), economy.total_balance)}${moneyField("transferBudget", tr("transferBudget"), economy.transfer_budget)}${moneyField("salaryBudget", tr("salaryBudget"), economy.salary_budget)}</div><button id="applyEconomy" class="primary">${h(tr("applyEconomy"))}</button></article>`; bindMoneyInputs(content);
    $("#applyEconomy").onclick = async () => { try { await window.tfm2.request("APPLY_EDITOR_SETTINGS", { economy: { total_balance: parseMoney($("#money").value), transfer_budget: parseMoney($("#transferBudget").value), salary_budget: parseMoney($("#salaryBudget").value) } }); toast(tr("applied")); } catch (error) { toast(error.message, true); } };
  } catch (error) { content.innerHTML = `<div class="empty">${h(error.message)}</div>`; }
}

async function lockEntirePlayer() {
  try {
    const player = await applyPlayer({ quiet: true });
    const mastery = await normalizedMastery(player.id); const masteryRows = [...mastery.active, ...mastery.inactive];
    const keyed = PLAYER_STAT_KEYS.map((key, index) => [`stat.${key}`, player.stats[index]]);
    keyed.push(...POSITION_KEYS.map((key, index) => [`stat.${key}`, player.positions[index]]), ["hidden.potential", player.potential], ["management.stamina", player.stamina], ["management.stress", player.stress], ["management.condition", player.condition], ["age", player.age]);
    keyed.push(...Object.entries(player.communication).filter(([, value]) => value > 0).map(([region, value]) => [`stat.language.${region}`, value]));
    keyed.push(...masteryRows.map((row) => [`champion_proficiency.${row.champion_id}`, row.value]));
    await window.tfm2.request("SET_LOCK", { target_id: player.id, target_name: player.name, group: "player_profile", value_keys: keyed.map(([key]) => key), values: keyed.map(([, value]) => value), status: "active", error: null });
    toast(tr("locked")); render();
  } catch (error) { toast(error.message, true); }
}

async function lockEntireStaff() {
  try {
    const staff = await applyStaff({ quiet: true }); const keyed = STAFF_STAT_KEYS.map((key, index) => [`stat.${key}`, staff.stats[index]]);
    keyed.push(["age", staff.age], ...Object.entries(staff.communication).filter(([, value]) => value > 0).map(([region, value]) => [`language.${region}`, value]));
    await window.tfm2.request("SET_LOCK", { target_id: staff.id, target_name: staff.name, group: "staff_profile", value_keys: keyed.map(([key]) => key), values: keyed.map(([, value]) => value), status: "active", error: null });
    toast(tr("locked")); render();
  } catch (error) { toast(error.message, true); }
}

async function loadLocks() { try { state.locks = await window.tfm2.request("GET_LOCKS"); render(); } catch (error) { toast(error.message, true); } }
function renderLocks(content) { content.innerHTML = `<div class="table-card"><div class="table-scroll"><table><thead><tr><th>${h(tr("target"))}</th><th>${h(tr("group"))}</th><th>${h(tr("values"))}</th><th></th></tr></thead><tbody>${state.locks.map((lock) => `<tr><td>${h(lock.target_name)} #${lock.target_id}</td><td>${h(lock.group)}</td><td>${lock.values.length.toLocaleString()} values</td><td><button class="danger small" data-unlock="${lock.target_id}" data-group="${h(lock.group)}">${h(tr("unlock"))}</button></td></tr>`).join("") || `<tr><td colspan="4">${h(tr("noData"))}</td></tr>`}</tbody></table></div></div>`; content.querySelectorAll("[data-unlock]").forEach((button) => button.onclick = async () => { try { state.locks = await window.tfm2.request("UNLOCK", { target_id: Number(button.dataset.unlock), group: button.dataset.group }); render(); } catch (error) { toast(error.message, true); } }); }

async function normalizedMastery(athleteId) {
  const [mastery, catalog] = await Promise.all([window.tfm2.request("GET_PLAYER_MASTERY", { athlete_id: athleteId }), window.tfm2.request("GET_CATALOG")]);
  if (catalog.status !== "ready") throw new Error("champion_catalog_not_ready");
  const byId = new Map([...mastery.active, ...mastery.inactive].map((row) => [row.champion_id, row])); const activeIds = new Set(catalog.champions.map((row) => row.champion_id));
  const active = catalog.champions.map((champion) => ({ ...(byId.get(champion.champion_id) || { champion_id: champion.champion_id, value: 0, floor: 0 }), display_name: champion.display_name, active: true }));
  const inactive = [...byId.values()].filter((row) => !activeIds.has(row.champion_id)).map((row) => ({ ...row, active: false }));
  const sortRows = (rows) => rows.sort((a, b) => a.display_name.localeCompare(b.display_name) || a.champion_id.localeCompare(b.champion_id));
  return { athlete_id: athleteId, active: sortRows(active), inactive: sortRows(inactive) };
}

async function openMastery() {
  try {
    const mastery = await normalizedMastery(state.player.id); const rows = [...mastery.active, ...mastery.inactive];
    $("#modal").innerHTML = `<div class="card-heading"><div><strong>${h(tr("mastery"))} · ${h(state.player.name)}</strong></div><button id="closeModal" class="secondary small">×</button></div><div class="mastery-scroll"><section class="mastery-section active"><h3>${h(tr("activeChampions"))} · ${mastery.active.length}</h3>${masteryGrid(mastery.active)}</section><section class="mastery-section inactive"><h3>${h(tr("inactiveChampions"))} · ${mastery.inactive.length}</h3>${masteryGrid(mastery.inactive)}</section></div><div class="modal-actions"><button id="maxMastery" class="secondary">${h(tr("maxAll"))}</button><button id="saveMastery" class="primary">${h(tr("saveMastery"))}</button></div>`;
    $("#modalBackdrop").hidden = false; $("#closeModal").onclick = closeModal; $("#maxMastery").onclick = () => document.querySelectorAll("[data-mastery-id]").forEach((input) => { input.value = 100; });
    $("#saveMastery").onclick = async () => { try {
      const values = rows.map((row) => ({ champion_id: row.champion_id, value: clampInt(document.querySelector(`[data-mastery-id="${CSS.escape(row.champion_id)}"]`).value) }));
      const readback = await window.tfm2.request("APPLY_PLAYER_EDIT", { athlete_id: state.player.id, mastery: values });
      values.forEach(({ champion_id }) => {
        const raw = Number(readback.record?.champion_proficiency?.[champion_id]?.value);
        if (!Number.isFinite(raw)) throw new Error(`mastery_readback_missing:${champion_id}`);
      });
      closeModal(); toast(tr("applied"));
    } catch (error) { toast(error.message, true); } };
  } catch (error) { toast(error.message, true); }
}
function masteryGrid(rows) { return `<div class="mastery-grid">${rows.map((row) => `<label class="mastery-item"><span><strong>${h(row.display_name)}</strong><small title="${h(row.champion_id)}">${h(row.champion_id)}</small></span><input data-mastery-id="${h(row.champion_id)}" type="number" min="0" max="100" value="${row.value}"></label>`).join("") || `<p>${h(tr("noData"))}</p>`}</div>`; }
function closeModal() { $("#modalBackdrop").hidden = true; $("#modal").innerHTML = ""; }

$("#search").oninput = (event) => { state.search = event.target.value; renderEntityList(); };
$("#sidebarToggle").onclick = () => { state.sidebarHidden = true; localStorage.setItem("tfm2.editor.sidebarHidden", "1"); render(); };
$("#sidebarReveal").onclick = () => { state.sidebarHidden = false; localStorage.setItem("tfm2.editor.sidebarHidden", "0"); render(); };
$("#modalBackdrop").onclick = (event) => { if (event.target === $("#modalBackdrop")) closeModal(); };
document.querySelectorAll("[data-language]").forEach((button) => button.onclick = async () => { state.language = button.dataset.language; await window.tfm2.setLanguage(state.language); render(); });

(async () => {
  const settings = await window.tfm2.settings();
  state.language = settings.language;
  await loadLists();
  window.tfm2.onStateChanged((message) => {
    if (document.visibilityState === "hidden") return;
    if (EditorModel.shouldRefreshForScopes(message?.scopes)) loadLists({ silent: true });
  });
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "visible") loadLists({ silent: true });
  });
})();
