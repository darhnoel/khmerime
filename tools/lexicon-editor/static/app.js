const state = {
  meta: null,
  table: null,
  page: 1,
  pageSize: 100,
  lastPage: 1,
  total: 0,
  searchTimer: null,
  activeRowId: null,
  activeTab: "grid-panel",
  gridScrollTop: 0,
  gridScrollLeft: 0,
  selectedRowIds: new Set(),
};

const $ = (id) => document.getElementById(id);
const STORAGE_KEY = "khmerime.lexiconEditor.view.v1";

function readSavedView() {
  try {
    return JSON.parse(window.localStorage.getItem(STORAGE_KEY) || "{}");
  } catch (_error) {
    return {};
  }
}

function writeSavedView() {
  const view = {
    query: $("query-input")?.value || "",
    chunk: $("chunk-filter")?.value || "",
    status: $("status-filter")?.value || "",
    category: $("category-filter")?.value || "",
    page: state.page,
    pageSize: state.pageSize,
    activeRowId: state.activeRowId,
    activeTab: state.activeTab,
    gridScrollTop: state.gridScrollTop,
    gridScrollLeft: state.gridScrollLeft,
  };
  window.localStorage.setItem(STORAGE_KEY, JSON.stringify(view));
}

function restoreSelectValue(id, value) {
  if (value === undefined || value === null) return;
  const node = $(id);
  if ([...node.options].some((option) => option.value === value)) {
    node.value = value;
  }
}

function restoreSavedViewControls() {
  const saved = readSavedView();
  if (typeof saved.query === "string") $("query-input").value = saved.query;
  restoreSelectValue("chunk-filter", saved.chunk);
  restoreSelectValue("status-filter", saved.status);
  restoreSelectValue("category-filter", saved.category);
  if ([50, 100, 250].includes(Number(saved.pageSize))) {
    state.pageSize = Number(saved.pageSize);
    $("page-size").value = String(state.pageSize);
  }
  if (Number.isInteger(Number(saved.page)) && Number(saved.page) > 0) {
    state.page = Number(saved.page);
  }
  if (typeof saved.activeRowId === "string") state.activeRowId = saved.activeRowId;
  if (typeof saved.activeTab === "string") state.activeTab = saved.activeTab;
  state.gridScrollTop = Number(saved.gridScrollTop) || 0;
  state.gridScrollLeft = Number(saved.gridScrollLeft) || 0;
}

function showMessage(text, timeout = 4200) {
  const node = $("message");
  node.textContent = text;
  node.classList.add("visible");
  if (timeout) {
    window.clearTimeout(showMessage.timer);
    showMessage.timer = window.setTimeout(() => node.classList.remove("visible"), timeout);
  }
}

async function api(path, options = {}) {
  const init = { ...options };
  if (init.body && typeof init.body !== "string") {
    init.body = JSON.stringify(init.body);
    init.headers = { "Content-Type": "application/json", ...(init.headers || {}) };
  }
  const response = await fetch(path, init);
  const payload = await response.json();
  if (!response.ok) {
    throw new Error(payload.error || response.statusText);
  }
  return payload;
}

function optionList(values, includeAll = false) {
  const prefix = includeAll ? [{ label: "all", value: "" }] : [];
  return prefix.concat(values.map((value) => ({ label: value, value })));
}

function fillSelect(select, values, includeAll = true) {
  select.replaceChildren();
  for (const item of optionList(values, includeAll)) {
    const option = document.createElement("option");
    option.value = item.value;
    option.textContent = item.label;
    select.appendChild(option);
  }
}

function updateBulkValues() {
  const column = $("bulk-column").value;
  const values = {
    category: state.meta.categories,
    status: state.meta.statuses,
    freq_lang: state.meta.freq_langs,
  }[column];
  fillSelect($("bulk-value"), values, false);
}

function selectedIds() {
  return [...state.selectedRowIds];
}

function selectedData() {
  const selected = state.selectedRowIds;
  return state.table.getData().filter((row) => selected.has(row.id));
}

function selectedOrActiveIds() {
  const ids = selectedIds();
  if (ids.length) return ids;
  return state.activeRowId ? [state.activeRowId] : [];
}

function filters() {
  return {
    query: $("query-input").value.trim(),
    chunk: $("chunk-filter").value,
    status: $("status-filter").value,
    category: $("category-filter").value,
  };
}

function movementBlocked() {
  const current = filters();
  return Boolean(current.query || current.status || current.category);
}

async function loadMeta() {
  state.meta = await api("/api/meta");
  fillSelect($("chunk-filter"), state.meta.chunks, true);
  fillSelect($("status-filter"), state.meta.statuses, true);
  fillSelect($("category-filter"), state.meta.categories, true);
  restoreSavedViewControls();
  updateBulkValues();
  renderDirty();
}

function statusText() {
  if (!state.meta) return "Loading...";
  const dirty = state.meta.dirty_chunks.length ? `Dirty: ${state.meta.dirty_chunks.join(", ")}` : "No dirty chunks";
  const external = state.meta.external_changes.length ? ` External changes: ${state.meta.external_changes.join(", ")}` : "";
  return `${dirty}.${external}`;
}

function renderDirty() {
  $("status-line").textContent = statusText();
  $("undo-button").disabled = !state.meta.can_undo;
  $("redo-button").disabled = !state.meta.can_redo;
  const lines = [
    `Dirty chunks: ${state.meta.dirty_chunks.length ? state.meta.dirty_chunks.join(", ") : "(none)"}`,
    `Undo available: ${state.meta.can_undo ? "yes" : "no"}`,
    `Redo available: ${state.meta.can_redo ? "yes" : "no"}`,
  ];
  if (state.meta.external_changes.length) {
    lines.push(`External changes: ${state.meta.external_changes.join(", ")}`);
  }
  $("dirty-output").textContent = lines.join("\n");
}

function params() {
  const current = filters();
  const query = new URLSearchParams({
    page: String(state.page),
    page_size: String(state.pageSize),
    query: current.query,
    chunk: current.chunk,
    status: current.status,
    category: current.category,
  });
  return query.toString();
}

async function loadRows() {
  const payload = await api(`/api/rows?${params()}`);
  state.lastPage = payload.last_page;
  state.total = payload.total;
  if (state.page > state.lastPage) {
    state.page = state.lastPage;
    return loadRows();
  }
  const pageIds = new Set(payload.data.map((row) => row.id));
  for (const id of [...state.selectedRowIds]) {
    if (!pageIds.has(id) && id.startsWith("new:")) {
      state.selectedRowIds.delete(id);
    }
  }
  await state.table.setData(payload.data);
  restoreGridScroll();
  $("page-label").textContent = `Page ${state.page} / ${state.lastPage}`;
  $("total-label").textContent = `${state.total} rows`;
  $("prev-page").disabled = state.page <= 1;
  $("next-page").disabled = state.page >= state.lastPage;
  writeSavedView();
}

function tableHolder() {
  return document.querySelector("#grid .tabulator-tableholder");
}

function captureGridScroll() {
  const holder = tableHolder();
  if (!holder) return;
  state.gridScrollTop = holder.scrollTop;
  state.gridScrollLeft = holder.scrollLeft;
  writeSavedView();
}

function restoreGridScroll() {
  window.requestAnimationFrame(() => {
    const holder = tableHolder();
    if (!holder) return;
    holder.scrollTop = state.gridScrollTop;
    holder.scrollLeft = state.gridScrollLeft;
  });
}

function runtimeFormatter(cell) {
  const value = cell.getValue();
  const klass = value === "included" ? "runtime-included" : "runtime-excluded";
  return `<span class="${klass}">${value}</span>`;
}

function issueFormatter(cell) {
  const value = cell.getValue() || [];
  if (!value.length) return "";
  return `<span class="cell-list">${value.join("<br>")}</span>`;
}

function makeTable() {
  state.table = new Tabulator("#grid", {
    height: "62vh",
    layout: "fitDataStretch",
    index: "id",
    selectableRows: "highlight",
    editTriggerEvent: "click",
    reactiveData: false,
    rowFormatter(row) {
      const data = row.getData();
      row.getElement().classList.toggle("row-invalid", Boolean(data.errors && data.errors.length));
      row.getElement().classList.toggle("row-warning", Boolean(data.warnings && data.warnings.length));
      row.getElement().classList.toggle("row-dirty", Boolean(data.dirty));
      row.getElement().classList.toggle("row-active", data.id === state.activeRowId);
      row.getElement().classList.toggle("row-selected", state.selectedRowIds.has(data.id));
    },
    columns: [
      {
        formatter(cell) {
          const checked = state.selectedRowIds.has(cell.getRow().getData().id) ? "checked" : "";
          return `<input type="checkbox" aria-label="Select Row" ${checked}>`;
        },
        hozAlign: "center",
        headerSort: false,
        width: 44,
        cellClick(event, cell) {
          event.stopPropagation();
          const row = cell.getRow();
          const id = row.getData().id;
          if (state.selectedRowIds.has(id)) {
            state.selectedRowIds.delete(id);
          } else {
            state.selectedRowIds.add(id);
          }
          state.activeRowId = id;
          row.reformat();
        },
      },
      { title: "chunk", field: "chunk", width: 128, headerSort: false },
      { title: "row", field: "row", width: 70, headerSort: false },
      { title: "orig", field: "orig_line", width: 70, headerSort: false },
      { title: "runtime", field: "runtime", width: 92, formatter: runtimeFormatter, headerSort: false },
      { title: "roman", field: "roman", editor: "input", width: 170, headerSort: false },
      { title: "target", field: "target", editor: "input", width: 190, headerSort: false },
      { title: "freq", field: "freq", editor: "number", width: 82, headerSort: false, editorParams: { min: 1, step: 1 } },
      { title: "lang", field: "freq_lang", editor: "list", width: 90, headerSort: false, editorParams: () => ({ values: state.meta.freq_langs }) },
      { title: "category", field: "category", editor: "list", width: 128, headerSort: false, editorParams: () => ({ values: state.meta.categories }) },
      { title: "status", field: "status", editor: "list", width: 118, headerSort: false, editorParams: () => ({ values: state.meta.statuses }) },
      { title: "notes", field: "notes", editor: "input", minWidth: 220, headerSort: false },
      { title: "errors", field: "errors", formatter: issueFormatter, width: 190, headerSort: false },
      { title: "warnings", field: "warnings", formatter: issueFormatter, width: 230, headerSort: false },
    ],
  });

  state.table.on("cellEdited", async (cell) => {
    const field = cell.getField();
    const data = cell.getRow().getData();
    try {
      const payload = await api("/api/edit-cell", {
        method: "POST",
        body: { row_id: data.id, column: field, value: String(cell.getValue() ?? "") },
      });
      if (payload.meta) {
        state.meta = payload.meta;
        renderDirty();
      } else {
        await loadMeta();
      }
      await loadRows();
    } catch (error) {
      showMessage(error.message, 8000);
      await loadRows();
    }
  });
  state.table.on("rowClick", (_event, row) => {
    state.activeRowId = row.getData().id;
    document.querySelectorAll(".tabulator-row.row-active").forEach((node) => node.classList.remove("row-active"));
    row.getElement().classList.add("row-active");
    writeSavedView();
  });
  state.table.on("tableBuilt", () => {
    const holder = tableHolder();
    if (!holder) return;
    holder.addEventListener("scroll", () => {
      window.clearTimeout(captureGridScroll.timer);
      captureGridScroll.timer = window.setTimeout(captureGridScroll, 120);
    });
  });
}

async function postAction(path, body = {}, reload = true) {
  const payload = await api(path, { method: "POST", body });
  if (payload.meta) {
    state.meta = payload.meta;
    renderDirty();
  } else {
    await loadMeta();
  }
  if (reload) await loadRows();
  return payload;
}

async function addRow() {
  const rows = selectedData();
  const current = filters();
  const body = {};
  if (rows.length) {
    body.after_row_id = rows[0].id;
  } else if (state.activeRowId) {
    body.after_row_id = state.activeRowId;
  } else if (current.chunk) {
    body.chunk = current.chunk;
  } else {
    showMessage("Select a row or choose a chunk before adding.");
    return;
  }
  const payload = await api("/api/add-row", { method: "POST", body });
  if (payload.meta) {
    state.meta = payload.meta;
    renderDirty();
  }
  if (payload.row) {
    state.activeRowId = payload.row.id;
    state.selectedRowIds.clear();
    state.selectedRowIds.add(payload.row.id);
    $("chunk-filter").value = payload.row.chunk;
    $("query-input").value = "";
    $("status-filter").value = "";
    $("category-filter").value = "";
    state.page = Math.max(1, Math.ceil(Number(payload.row.row || 1) / state.pageSize));
    state.gridScrollTop = 0;
  }
  await loadRows();
  showMessage("Added a draft row.");
}

async function revertRows() {
  const ids = selectedOrActiveIds();
  if (!ids.length) return showMessage("Select rows or click a row first.");
  if (!window.confirm(`Revert ${ids.length} row(s)? New draft rows will be removed.`)) return;
  for (const id of ids) {
    await api("/api/revert-row", { method: "POST", body: { row_id: id } });
  }
  state.selectedRowIds.clear();
  state.activeRowId = null;
  await loadMeta();
  await loadRows();
  showMessage("Reverted selected/active row(s).");
}

async function softRemove() {
  const ids = selectedIds();
  if (!ids.length) return showMessage("Select rows first.");
  if (!window.confirm(`Soft remove ${ids.length} selected row(s)?`)) return;
  await postAction("/api/soft-remove", { row_ids: ids });
}

async function moveRows(direction) {
  const ids = selectedIds();
  if (!ids.length) return showMessage("Select rows first.");
  if (movementBlocked()) return showMessage("Clear text/status/category filters before moving rows.");
  await postAction("/api/move-rows", { row_ids: ids, direction });
}

async function bulkEdit() {
  const ids = selectedOrActiveIds();
  if (!ids.length) return showMessage("Select rows or click a row first.");
  await postAction("/api/bulk-edit", {
    row_ids: ids,
    column: $("bulk-column").value,
    value: $("bulk-value").value,
  });
  showMessage(`Applied ${$("bulk-column").value}=${$("bulk-value").value} to ${ids.length} row(s).`);
}

async function saveBuildCheck() {
  try {
    const payload = await postAction("/api/save-build-check", {}, false);
    if (payload.diff !== undefined) {
      $("diff-output").textContent = payload.diff || "(no diff)";
      switchTab("diff-panel");
    }
    await loadRows();
    showMessage(`${payload.message}${payload.backup_dir ? `\nBackup: ${payload.backup_dir}` : ""}`, 7000);
  } catch (error) {
    showMessage(error.message, 12000);
  }
}

async function loadDiff() {
  const payload = await api("/api/diff");
  $("diff-output").textContent = payload.diff || "(no diff)";
}

async function loadProblems() {
  const payload = await api("/api/problems");
  $("problems-count").textContent = `${payload.total} problem entries`;
  const list = $("problems-list");
  list.replaceChildren();
  for (const item of payload.problems) {
    const row = item.row;
    const element = document.createElement("div");
    element.className = "problem-item";
    const type = document.createElement("div");
    type.className = "problem-type";
    type.textContent = item.type;
    const detail = document.createElement("div");
    detail.textContent = `${row.chunk} row ${row.row} ${row.roman} -> ${row.target}`;
    const open = document.createElement("button");
    open.type = "button";
    open.textContent = "Open";
    open.addEventListener("click", async () => {
      $("chunk-filter").value = row.chunk;
      $("query-input").value = "";
      $("status-filter").value = "";
      $("category-filter").value = "";
      state.activeRowId = row.id;
      state.page = Math.max(1, Math.ceil(Number(row.row || 1) / state.pageSize));
      state.gridScrollTop = 0;
      switchTab("grid-panel");
      await loadRows();
    });
    element.append(type, detail, open);
    list.appendChild(element);
  }
}

function switchTab(panelId) {
  state.activeTab = panelId;
  document.querySelectorAll(".tab").forEach((node) => node.classList.toggle("active", node.dataset.tab === panelId));
  document.querySelectorAll(".panel").forEach((node) => node.classList.toggle("active", node.id === panelId));
  writeSavedView();
}

function wireEvents() {
  document.querySelectorAll(".tab").forEach((node) => node.addEventListener("click", () => switchTab(node.dataset.tab)));
  $("query-input").addEventListener("input", () => {
    window.clearTimeout(state.searchTimer);
    state.searchTimer = window.setTimeout(() => {
      state.page = 1;
      state.gridScrollTop = 0;
      loadRows().catch((error) => showMessage(error.message));
    }, 180);
  });
  ["chunk-filter", "status-filter", "category-filter"].forEach((id) => {
    $(id).addEventListener("change", () => {
      state.page = 1;
      state.gridScrollTop = 0;
      loadRows().catch((error) => showMessage(error.message));
    });
  });
  $("page-size").addEventListener("change", () => {
    state.pageSize = Number($("page-size").value);
    state.page = 1;
    state.gridScrollTop = 0;
    loadRows().catch((error) => showMessage(error.message));
  });
  $("prev-page").addEventListener("click", () => {
    state.page = Math.max(1, state.page - 1);
    state.gridScrollTop = 0;
    loadRows().catch((error) => showMessage(error.message));
  });
  $("next-page").addEventListener("click", () => {
    state.page = Math.min(state.lastPage, state.page + 1);
    state.gridScrollTop = 0;
    loadRows().catch((error) => showMessage(error.message));
  });
  $("add-row-button").addEventListener("click", () => addRow().catch((error) => showMessage(error.message)));
  $("revert-row-button").addEventListener("click", () => revertRows().catch((error) => showMessage(error.message)));
  $("soft-remove-button").addEventListener("click", () => softRemove().catch((error) => showMessage(error.message)));
  $("move-up-button").addEventListener("click", () => moveRows("up").catch((error) => showMessage(error.message)));
  $("move-down-button").addEventListener("click", () => moveRows("down").catch((error) => showMessage(error.message)));
  $("move-top-button").addEventListener("click", () => moveRows("top").catch((error) => showMessage(error.message)));
  $("move-bottom-button").addEventListener("click", () => moveRows("bottom").catch((error) => showMessage(error.message)));
  $("bulk-column").addEventListener("change", updateBulkValues);
  $("bulk-apply-button").addEventListener("click", () => bulkEdit().catch((error) => showMessage(error.message)));
  $("undo-button").addEventListener("click", () => postAction("/api/undo").catch((error) => showMessage(error.message)));
  $("redo-button").addEventListener("click", () => postAction("/api/redo").catch((error) => showMessage(error.message)));
  $("save-button").addEventListener("click", saveBuildCheck);
  $("refresh-diff-button").addEventListener("click", () => loadDiff().catch((error) => showMessage(error.message)));
  $("refresh-problems-button").addEventListener("click", () => loadProblems().catch((error) => showMessage(error.message)));
  $("discard-button").addEventListener("click", async () => {
    if (!window.confirm("Discard all unsaved draft changes?")) return;
    await postAction("/api/discard-draft");
  });
  $("reload-button").addEventListener("click", async () => {
    let payload = await api("/api/reload", { method: "POST", body: {} });
    if (payload.needs_confirmation) {
      if (!window.confirm(`Discard unsaved draft changes in ${payload.dirty_chunks.join(", ")}?`)) return;
      payload = await api("/api/reload", { method: "POST", body: { force: true } });
    }
    if (payload.meta) {
      state.meta = payload.meta;
      renderDirty();
    } else {
      await loadMeta();
    }
    await loadRows();
  });

  window.addEventListener("beforeunload", writeSavedView);

  document.addEventListener("keydown", (event) => {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
      event.preventDefault();
      saveBuildCheck();
    } else if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "z") {
      event.preventDefault();
      postAction("/api/undo").catch((error) => showMessage(error.message));
    } else if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "y") {
      event.preventDefault();
      postAction("/api/redo").catch((error) => showMessage(error.message));
    } else if (event.key === "Delete" && selectedIds().length) {
      event.preventDefault();
      softRemove().catch((error) => showMessage(error.message));
    }
  });
}

async function init() {
  wireEvents();
  await loadMeta();
  makeTable();
  switchTab(state.activeTab || "grid-panel");
  await loadRows();
  await loadProblems();
}

init().catch((error) => showMessage(error.message, 0));
