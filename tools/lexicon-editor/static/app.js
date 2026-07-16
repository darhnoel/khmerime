const state = {
  meta: null,
  table: null,
  page: 1,
  pageSize: 100,
  lastPage: 1,
  total: 0,
  searchTimer: null,
  activeRowId: null,
  gridScrollTop: 0,
  gridScrollLeft: 0,
  selectedRowIds: new Set(),
  problems: null,
  regexApply: null,
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
    target: $("target-input")?.value || "",
    runtime: $("runtime-filter")?.value || "",
    filters: Object.fromEntries(FILTERS.map((f) => [f.key, f.values()])),
    page: state.page,
    pageSize: state.pageSize,
    activeRowId: state.activeRowId,
    gridScrollTop: state.gridScrollTop,
    gridScrollLeft: state.gridScrollLeft,
  };
  window.localStorage.setItem(STORAGE_KEY, JSON.stringify(view));
}

function restoreSavedViewControls() {
  const saved = readSavedView();
  if (typeof saved.query === "string") $("query-input").value = saved.query;
  if (typeof saved.target === "string") $("target-input").value = saved.target;
  if (["", "included", "excluded"].includes(saved.runtime)) $("runtime-filter").value = saved.runtime;
  const savedFilters = saved.filters || {};
  for (const f of FILTERS) f.set(savedFilters[f.key] || []);
  if ([50, 100, 250, 500, 1000, 1500, 2000].includes(Number(saved.pageSize))) {
    state.pageSize = Number(saved.pageSize);
    $("page-size").value = String(state.pageSize);
  }
  if (Number.isInteger(Number(saved.page)) && Number(saved.page) > 0) {
    state.page = Number(saved.page);
  }
  if (typeof saved.activeRowId === "string") state.activeRowId = saved.activeRowId;
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
    const error = new Error(payload.error || response.statusText);
    error.detail = payload.detail;
    throw error;
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

function refreshSelectionUI() {
  const count = selectedOrActiveIds().length;
  const selected = selectedIds().length;
  $("selection-count").textContent = selected
    ? `${selected} selected`
    : count
      ? "1 active"
      : "0 selected";
}

const POPOVERS = {
  "set-popover": "set-open-button",
  "overflow-popover": "overflow-open-button",
  "chunk-filter-popover": "chunk-filter-button",
  "status-filter-popover": "status-filter-button",
  "category-filter-popover": "category-filter-button",
};

function closePopovers(except) {
  for (const [id, trigger] of Object.entries(POPOVERS)) {
    if (id === except) continue;
    $(id).hidden = true;
    $(trigger).setAttribute("aria-expanded", "false");
  }
}

function applyFilters() {
  state.page = 1;
  state.gridScrollTop = 0;
  loadRows().catch((error) => showMessage(error.message));
}

// Reset all filters and scope the view to a single chunk (used when jumping
// to a specific row from add/duplicate/problem-open).
function focusChunk(name) {
  $("query-input").value = "";
  $("target-input").value = "";
  $("runtime-filter").value = "";
  for (const f of FILTERS) f.set(f.key === "chunk" ? [name] : []);
}

// One multi-select dropdown filter (chunk, status, category). Owns a Set of
// selected values and renders a tristate "All" header + a checkbox list.
function makeMultiFilter({ key, prefix, allLabel, noun, options }) {
  const state_set = new Set();
  const el = (suffix) => $(`${prefix}-${suffix}`);
  const label = () => {
    const n = state_set.size;
    if (n === 0) return allLabel;
    if (n === 1) return [...state_set][0];
    return `${n} ${noun}`;
  };
  const render = () => {
    const all = options();
    const selected = state_set.size;
    el("button").textContent = label();
    const allBox = el("all");
    allBox.checked = selected > 0 && selected === all.length;
    allBox.indeterminate = selected > 0 && selected < all.length;
    el("count").textContent = selected ? `${selected} / ${all.length}` : "";
    const list = el("list");
    list.replaceChildren();
    for (const name of all) {
      const row = document.createElement("label");
      row.className = "chunk-filter-row";
      const box = document.createElement("input");
      box.type = "checkbox";
      box.checked = state_set.has(name);
      box.addEventListener("change", () => {
        box.checked ? state_set.add(name) : state_set.delete(name);
        render();
        applyFilters();
      });
      const text = document.createElement("span");
      text.textContent = name;
      row.append(box, text);
      list.appendChild(row);
    }
  };
  const set = (values) => {
    const valid = new Set(options());
    state_set.clear();
    for (const v of values || []) if (valid.has(v)) state_set.add(v);
    if (state.meta) render();
  };
  const wire = () => {
    el("button").addEventListener("click", (event) => {
      event.stopPropagation();
      togglePopover(`${prefix}-popover`, `${prefix}-button`);
    });
    el("popover").addEventListener("click", (event) => event.stopPropagation());
    el("all").addEventListener("click", (event) => {
      const selectAll = state_set.size < options().length;
      set(selectAll ? options() : []);
      event.target.checked = selectAll;
      applyFilters();
    });
  };
  return { key, prefix, values: () => [...state_set], set, render, wire, popoverId: `${prefix}-popover`, buttonId: `${prefix}-button` };
}

const FILTERS = [
  makeMultiFilter({ key: "chunk", prefix: "chunk-filter", allLabel: "All chunks", noun: "chunks", options: () => state.meta.chunks }),
  makeMultiFilter({ key: "status", prefix: "status-filter", allLabel: "All status", noun: "statuses", options: () => state.meta.statuses }),
  makeMultiFilter({ key: "category", prefix: "category-filter", allLabel: "All categories", noun: "categories", options: () => state.meta.categories }),
];
const filterByKey = (key) => FILTERS.find((f) => f.key === key);

function togglePopover(popoverId, triggerId) {
  const popover = $(popoverId);
  const willOpen = popover.hidden;
  closePopovers(willOpen ? popoverId : null);
  popover.hidden = !willOpen;
  $(triggerId).setAttribute("aria-expanded", String(willOpen));
}

function openContextMenu(pageX, pageY) {
  // Reuse the overflow menu at the cursor — same items, same handlers.
  closePopovers("overflow-popover");
  const menu = $("overflow-popover");
  menu.classList.add("context-menu");
  menu.hidden = false;
  const rect = menu.getBoundingClientRect();
  const x = Math.min(pageX, window.innerWidth - rect.width - 8);
  const y = Math.min(pageY, window.innerHeight - rect.height - 8);
  menu.style.left = `${Math.max(8, x)}px`;
  menu.style.top = `${Math.max(8, y)}px`;
}

function movementBlocked() {
  const anyFilter = FILTERS.some((f) => f.values().length);
  return Boolean($("query-input").value.trim() || $("target-input").value.trim() || $("runtime-filter").value || anyFilter);
}

async function loadMeta() {
  state.meta = await api("/api/meta");
  restoreSavedViewControls();
  for (const f of FILTERS) f.render();
  updateBulkValues();
  renderDirty();
}

function renderDirty() {
  const edited = state.meta.edited_rows || 0;
  $("save-label").textContent = edited ? `Build (${edited})` : "Build";
  $("save-button").classList.toggle("has-edits", edited > 0);
  $("save-button").title = edited
    ? `Save Build Check — ${edited} edited row(s) in ${state.meta.dirty_chunks.join(", ")}`
    : "Save Build Check";
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
  const query = new URLSearchParams({
    page: String(state.page),
    page_size: String(state.pageSize),
    query: $("query-input").value.trim(),
    target: $("target-input").value.trim(),
    runtime: $("runtime-filter").value,
  });
  for (const f of FILTERS) for (const v of f.values()) query.append(f.key, v);
  return query.toString();
}

// The filter the grid is currently showing, as a plain object (for bulk ops).
function currentFilter() {
  const filter = {
    query: $("query-input").value.trim(),
    target: $("target-input").value.trim(),
    runtime: $("runtime-filter").value,
  };
  for (const f of FILTERS) filter[f.key] = f.values();
  return filter;
}

// Regex scope: explicit checked/active rows win; otherwise the whole filtered set.
function regexScope() {
  const ids = selectedOrActiveIds();
  return ids.length ? { row_ids: ids } : { filter: currentFilter() };
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
  refreshSelectionUI();
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
    height: "100%",
    layout: "fitColumns",
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
        titleFormatter() {
          const pageIds = state.table ? state.table.getData().map((row) => row.id) : [];
          const allChecked = pageIds.length && pageIds.every((id) => state.selectedRowIds.has(id));
          return `<input type="checkbox" aria-label="Select all rows on page" tabindex="-1" style="pointer-events:none" ${allChecked ? "checked" : ""}>`;
        },
        headerClick(event, column) {
          event.stopPropagation();
          const pageIds = state.table.getData().map((row) => row.id);
          const allChecked = pageIds.length && pageIds.every((id) => state.selectedRowIds.has(id));
          for (const id of pageIds) {
            if (allChecked) state.selectedRowIds.delete(id);
            else state.selectedRowIds.add(id);
          }
          // redraw(true) repaints body cells but not the header formatter — update the header box directly.
          const headerBox = column.getElement().querySelector("input[type=checkbox]");
          if (headerBox) headerBox.checked = !allChecked;
          state.table.redraw(true);
          refreshSelectionUI();
        },
        formatter(cell) {
          const checked = state.selectedRowIds.has(cell.getRow().getData().id) ? "checked" : "";
          // Display only: cellClick owns the state, so block the native toggle to avoid double-flip.
          return `<input type="checkbox" aria-label="Select Row" tabindex="-1" style="pointer-events:none" ${checked}>`;
        },
        hozAlign: "center",
        headerHozAlign: "center",
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
          refreshSelectionUI();
        },
      },
      { title: "chunk", field: "chunk", width: 120, headerSort: false },
      { title: "row", field: "row", width: 62, headerSort: false },
      { title: "orig", field: "orig_line", width: 62, headerSort: false },
      { title: "runtime", field: "runtime", width: 84, formatter: runtimeFormatter, headerSort: false },
      { title: "roman", field: "roman", editor: "input", widthGrow: 2, minWidth: 150, headerSort: false },
      { title: "target", field: "target", editor: "input", widthGrow: 2, minWidth: 150, headerSort: false },
      { title: "freq", field: "freq", editor: "number", width: 72, headerSort: false, editorParams: { min: 1, step: 1 } },
      { title: "lang", field: "freq_lang", editor: "list", width: 78, headerSort: false, editorParams: () => ({ values: state.meta.freq_langs }) },
      { title: "category", field: "category", editor: "list", width: 116, headerSort: false, editorParams: () => ({ values: state.meta.categories }) },
      { title: "status", field: "status", editor: "list", width: 100, headerSort: false, editorParams: () => ({ values: state.meta.statuses }) },
      { title: "notes", field: "notes", editor: "input", widthGrow: 3, minWidth: 200, headerSort: false },
      { title: "errors", field: "errors", formatter: issueFormatter, widthGrow: 1, minWidth: 120, headerSort: false },
      { title: "warnings", field: "warnings", formatter: issueFormatter, widthGrow: 1, minWidth: 140, headerSort: false },
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
      }
      // Update only the edited row in place — no full-page reload/repaint.
      // Cross-row warnings stay lazy: they refresh on save, filter change, or
      // reload, not on every keystroke.
      if (payload.row) {
        cell.getRow().update(payload.row);
        // Auto-accumulate genuinely-modified rows into the selection so a
        // batch status change targets exactly the rows you edited.
        if (payload.row.dirty && !state.selectedRowIds.has(payload.row.id)) {
          state.selectedRowIds.add(payload.row.id);
          cell.getRow().reformat();
          refreshSelectionUI();
        }
      }
    } catch (error) {
      showMessage(error.message, 8000);
      await loadRows();
    }
  });
  state.table.on("rowClick", (_event, row) => {
    state.activeRowId = row.getData().id;
    document.querySelectorAll(".tabulator-row.row-active").forEach((node) => node.classList.remove("row-active"));
    row.getElement().classList.add("row-active");
    refreshSelectionUI();
    writeSavedView();
  });
  state.table.on("rowContext", (event, row) => {
    event.preventDefault();
    const id = row.getData().id;
    if (!state.selectedRowIds.has(id)) state.activeRowId = id;
    row.getElement().classList.add("row-active");
    refreshSelectionUI();
    openContextMenu(event.pageX, event.pageY);
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
  closePopovers(null);
  if (payload.meta) {
    state.meta = payload.meta;
    renderDirty();
  } else {
    await loadMeta();
  }
  if (reload) await loadRows();
  refreshSelectionUI();
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
  } else if (current.chunks.length === 1) {
    body.chunk = current.chunks[0];
  } else {
    showMessage("Select a row or filter to a single chunk before adding.");
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
    focusChunk(payload.row.chunk);
    state.page = Math.max(1, Math.ceil(Number(payload.row.row || 1) / state.pageSize));
    state.gridScrollTop = 0;
  }
  await loadRows();
  showMessage("Added a draft row.");
}

async function duplicateRow() {
  const rows = selectedData();
  const rowId = rows.length ? rows[0].id : state.activeRowId;
  if (!rowId) {
    showMessage("Select or click a row to duplicate.");
    return;
  }
  const payload = await api("/api/duplicate-row", { method: "POST", body: { row_id: rowId } });
  if (payload.meta) {
    state.meta = payload.meta;
    renderDirty();
  }
  if (payload.row) {
    state.activeRowId = payload.row.id;
    state.selectedRowIds.clear();
    state.selectedRowIds.add(payload.row.id);
    focusChunk(payload.row.chunk);
    state.page = Math.max(1, Math.ceil(Number(payload.row.row || 1) / state.pageSize));
    state.gridScrollTop = 0;
  }
  await loadRows();
  showMessage("Duplicated row.");
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
  state.selectedRowIds.clear();
  state.activeRowId = null;
  refreshSelectionUI();
}

async function deleteRows() {
  const ids = selectedOrActiveIds();
  if (!ids.length) return showMessage("Select rows or click a row first.");
  await postAction("/api/delete-rows", { row_ids: ids });
  state.selectedRowIds.clear();
  state.activeRowId = null;
  showMessage(`Deleted ${ids.length} row(s). Undo (Ctrl+Z) or Save-time backup restores them.`);
}

// Predefined roman-normalization rules, applied in list order (overlaps like
// tteak/teak are ordered so the longer one wins first).
const REGEX_RULES = [
  { pattern: "tteak", replacement: "tta" },
  { pattern: "veak", replacement: "va" },
  { pattern: "yeak", replacement: "ya" },
  { pattern: "neak", replacement: "na" },
  { pattern: "teak", replacement: "ta" },
  { pattern: "geak", replacement: "ga" },
  { pattern: "jeak", replacement: "ja" },
  { pattern: "peak", replacement: "pa" },
  { pattern: "reak", replacement: "ra" },
  { pattern: "leak", replacement: "la" },
  { pattern: "vva", replacement: "va" },
  { pattern: "mma", replacement: "ma" },
  { pattern: "oanh", replacement: "anh" },
  { pattern: "uum", replacement: "um" },
  { pattern: "meak", replacement: "ma" },
  { pattern: "oach", replacement: "ach" },
  { pattern: "oam", replacement: "am" },
  { pattern: "oan", replacement: "an" },
  { pattern: "au$", replacement: "ov" },
  { pattern: "^au", replacement: "av" },
  { pattern: "ros$", replacement: "ruos" },
];

const RULE_ORDER_KEY = "khmerime.lexiconEditor.ruleOrder.v1";

function loadRuleOrder() {
  try {
    const saved = JSON.parse(window.localStorage.getItem(RULE_ORDER_KEY) || "[]");
    // Keep only valid indices, then append any new/missing rules at the end.
    const valid = saved.filter((i) => Number.isInteger(i) && i >= 0 && i < REGEX_RULES.length);
    const seen = new Set(valid);
    for (let i = 0; i < REGEX_RULES.length; i++) if (!seen.has(i)) valid.push(i);
    return [...new Set(valid)];
  } catch (_e) {
    return REGEX_RULES.map((_, i) => i);
  }
}

function saveRuleOrder(order) {
  window.localStorage.setItem(RULE_ORDER_KEY, JSON.stringify(order));
}

function renderRegexRules() {
  const list = $("regex-rules-list");
  list.replaceChildren();
  for (const index of loadRuleOrder()) {
    const rule = REGEX_RULES[index];
    const row = document.createElement("label");
    row.className = "chunk-filter-row regex-rule-row";
    row.draggable = true;
    row.dataset.index = String(index);
    const handle = document.createElement("span");
    handle.className = "drag-handle";
    handle.textContent = "⠿";
    handle.setAttribute("aria-hidden", "true");
    const box = document.createElement("input");
    box.type = "checkbox";
    box.dataset.index = String(index);
    box.className = "regex-rule-box";
    const text = document.createElement("span");
    text.innerHTML = `<code>${rule.pattern}</code> → <code>${rule.replacement}</code>`;
    row.append(handle, box, text);
    wireRuleDrag(row, list);
    list.appendChild(row);
  }
  $("regex-rules-all").checked = false;
}

function wireRuleDrag(row, list) {
  row.addEventListener("dragstart", (e) => {
    row.classList.add("dragging");
    e.dataTransfer.effectAllowed = "move";
  });
  row.addEventListener("dragend", () => {
    row.classList.remove("dragging");
    saveRuleOrder([...list.querySelectorAll(".regex-rule-row")].map((r) => Number(r.dataset.index)));
    $("regex-apply-button").disabled = true;
  });
  row.addEventListener("dragover", (e) => {
    e.preventDefault();
    const dragging = list.querySelector(".dragging");
    if (!dragging || dragging === row) return;
    const rect = row.getBoundingClientRect();
    const after = e.clientY > rect.top + rect.height / 2;
    list.insertBefore(dragging, after ? row.nextSibling : row);
  });
}

function selectedRules() {
  // querySelectorAll returns document (visual) order, which is the apply order.
  return [...document.querySelectorAll(".regex-rule-box")]
    .filter((box) => box.checked)
    .map((box) => REGEX_RULES[Number(box.dataset.index)]);
}

function regexBody() {
  return {
    ...regexScope(),
    column: $("regex-column").value,
    pattern: $("regex-pattern").value,
    replacement: $("regex-replacement").value,
  };
}

// Render preview changes as a table: old | new | Khmer target.
function renderPreviewTable(list, changes) {
  list.replaceChildren();
  if (!changes.length) {
    list.textContent = "No rows match — nothing would change.";
    return;
  }
  const table = document.createElement("table");
  table.className = "preview-table";
  table.innerHTML =
    "<thead><tr><th>from</th><th>to</th><th>target</th></tr></thead>";
  const body = document.createElement("tbody");
  for (const change of changes) {
    const tr = document.createElement("tr");
    const cells = [change.old, change.new, change.target || ""];
    for (const [i, text] of cells.entries()) {
      const td = document.createElement("td");
      td.textContent = text;
      if (i === 2) td.className = "preview-target";
      tr.appendChild(td);
    }
    body.appendChild(tr);
  }
  table.appendChild(body);
  list.appendChild(table);
}

async function regexRulesPreview() {
  const rules = selectedRules();
  if (!rules.length) return showMessage("Tick at least one rule.");
  const body = { ...regexScope(), column: "roman", rules };
  try {
    const payload = await api("/api/bulk-regex-rules-preview", { method: "POST", body });
    renderPreviewTable($("regex-preview-list"), payload.changes);
    $("regex-count").textContent = `${payload.count} row(s) would change`;
    state.regexApplyMode = "rules";
    state.regexApply = async () => {
      const result = await postAction("/api/bulk-regex-rules-apply", body);
      return result.updated;
    };
    $("regex-apply-button").disabled = !payload.count;
  } catch (error) {
    showMessage(error.message, 8000);
  }
}

async function regexPreview() {
  const body = regexBody();
  try {
    const payload = await api("/api/bulk-regex-preview", { method: "POST", body });
    renderPreviewTable($("regex-preview-list"), payload.changes);
    $("regex-count").textContent = `${payload.count} row(s) would change`;
    state.regexApplyMode = "manual";
    state.regexApply = async () => {
      const result = await postAction("/api/bulk-regex-apply", body);
      return result.updated;
    };
    $("regex-apply-button").disabled = !payload.count;
  } catch (error) {
    showMessage(error.message, 8000);
  }
}

async function regexApply() {
  if (!state.regexApply) return;
  try {
    const lastMode = state.regexApplyMode;
    const updated = await state.regexApply();
    refreshSelectionUI();
    showMessage(`Applied to ${updated} row(s).`);
    // Keep the modal, rule ticks, and selection (apply mutates in place — IDs stay valid),
    // then re-preview so Apply reflects the now-changed rows.
    $("regex-apply-button").disabled = true;
    state.regexApply = null;
    if (lastMode === "rules") await regexRulesPreview();
    else if (lastMode === "manual") await regexPreview();
  } catch (error) {
    showMessage(error.message, 8000);
  }
}

function openRegexModal() {
  $("regex-preview-list").replaceChildren();
  $("regex-count").textContent = "";
  $("regex-apply-button").disabled = true;
  state.regexApply = null;
  const selected = selectedOrActiveIds().length;
  $("regex-scope").textContent = selected
    ? `Applies to ${selected} selected row(s).`
    : `Applies to all ${state.total} row(s) matching the current filter.`;
  renderRegexRules();
  $("regex-modal").classList.add("visible");
  $("regex-pattern").focus();
}

function closeRegexModal() {
  $("regex-modal").classList.remove("visible");
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
  const column = $("bulk-column").value;
  const value = $("bulk-value").value;
  await postAction("/api/bulk-edit", { row_ids: ids, column, value });
  state.selectedRowIds.clear();
  state.activeRowId = null;
  refreshSelectionUI();
  showMessage(`Applied ${column}=${value} to ${ids.length} row(s).`);
}

async function saveBuildCheck() {
  try {
    const payload = await postAction("/api/save-build-check", {}, false);
    // Save drops drafts and rescans, regenerating every row ID. Any selection
    // held from before now points at IDs that no longer exist ("unknown row"),
    // so clear it before reloading the fresh grid.
    state.selectedRowIds.clear();
    state.activeRowId = null;
    if (payload.diff !== undefined) {
      $("diff-output").textContent = payload.diff || "(no diff)";
      $("diff-modal").classList.add("visible");
    }
    await loadRows();
    await loadProblems();
    showMessage(`${payload.message}${payload.backup_dir ? `\nBackup: ${payload.backup_dir}` : ""}`, 7000);
  } catch (error) {
    showBuildError(error);
  }
}

function showBuildError(error) {
  $("error-message").textContent = error.message;
  const holder = $("error-conflicts");
  holder.replaceChildren();
  const detail = error.detail || {};
  for (const dup of detail.duplicate_keys || []) {
    holder.appendChild(renderDuplicateFix(dup));
  }
  for (const conflict of detail.frequency_conflicts || []) {
    holder.appendChild(renderConflictFix(conflict));
  }
  $("error-modal").classList.add("visible");
}

function renderDuplicateFix(dup) {
  const card = document.createElement("div");
  card.className = "conflict-card";
  const head = document.createElement("div");
  head.className = "conflict-head";
  head.textContent = `${dup.roman} → ${dup.target} (${dup.freq_lang})`;
  card.appendChild(head);

  const detail = document.createElement("div");
  detail.className = "conflict-detail";
  detail.textContent = "Approved in two places — disable one to keep a single runtime entry.";
  card.appendChild(detail);

  const disableOne = async (rowId) => {
    try {
      await postAction("/api/bulk-edit", { row_ids: [rowId], column: "status", value: "disabled" });
      card.remove();
      showMessage(`Disabled a duplicate ${dup.roman}. Re-run Build.`);
      if (!$("error-conflicts").children.length) closeBuildError();
    } catch (e) {
      showMessage(e.message, 8000);
    }
  };
  for (const row of dup.rows) {
    const line = document.createElement("div");
    line.className = "conflict-actions";
    const label = document.createElement("span");
    label.className = "conflict-detail";
    label.textContent = `${row.chunk} (freq ${row.freq})`;
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "btn";
    btn.textContent = "disable this";
    btn.addEventListener("click", () => disableOne(row.id));
    line.append(label, btn);
    card.appendChild(line);
  }
  return card;
}

function renderConflictFix(conflict) {
  const card = document.createElement("div");
  card.className = "conflict-card";
  const head = document.createElement("div");
  head.className = "conflict-head";
  head.textContent = `${conflict.target} (${conflict.freq_lang})`;
  card.appendChild(head);

  const rowIds = conflict.rows.map((r) => r.id);
  const freqs = [...new Set(conflict.rows.map((r) => r.freq))];

  const detail = document.createElement("div");
  detail.className = "conflict-detail";
  detail.textContent = conflict.rows.map((r) => `${r.roman}=${r.freq}`).join(", ");
  card.appendChild(detail);

  const actions = document.createElement("div");
  actions.className = "conflict-actions";
  const apply = async (value) => {
    try {
      await postAction("/api/bulk-edit", { row_ids: rowIds, column: "freq", value: String(value) });
      card.remove();
      showMessage(`Set freq=${value} for ${conflict.target}. Re-run Build.`);
      if (!$("error-conflicts").children.length) closeBuildError();
    } catch (e) {
      showMessage(e.message, 8000);
    }
  };
  for (const freq of freqs) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "btn";
    btn.textContent = `use ${freq}`;
    btn.addEventListener("click", () => apply(freq));
    actions.appendChild(btn);
  }
  const input = document.createElement("input");
  input.type = "number";
  input.min = "1";
  input.placeholder = "freq";
  input.className = "conflict-input";
  const custom = document.createElement("button");
  custom.type = "button";
  custom.className = "btn primary";
  custom.textContent = "Set";
  custom.addEventListener("click", () => {
    if (input.value) apply(input.value);
  });
  actions.append(input, custom);
  card.appendChild(actions);
  return card;
}

function closeBuildError() {
  $("error-modal").classList.remove("visible");
}

async function loadDiff() {
  const payload = await api("/api/diff");
  $("diff-output").textContent = payload.diff || "(no diff)";
}

async function loadProblems() {
  state.problems = await api("/api/problems");
  renderProblems();
}

function renderProblems() {
  const payload = state.problems;
  if (!payload) return;
  const typeFilter = $("problem-type-filter").value;
  const shown = typeFilter ? payload.problems.filter((item) => item.type === typeFilter) : payload.problems;
  $("problems-count").textContent = `${shown.length} shown / ${payload.total} total`;
  const list = $("problems-list");
  list.replaceChildren();
  for (const item of shown) {
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
      focusChunk(row.chunk);
      state.activeRowId = row.id;
      state.page = Math.max(1, Math.ceil(Number(row.row || 1) / state.pageSize));
      state.gridScrollTop = 0;
      closeReview();
      await loadRows();
    });
    element.append(type, detail, open);
    list.appendChild(element);
  }
  updateReviewBadge();
}

async function openReview() {
  $("review-drawer").hidden = false;
  $("review-scrim").hidden = false;
  await loadProblems();
}

function closeReview() {
  $("review-drawer").hidden = true;
  $("review-scrim").hidden = true;
}

async function openDiff() {
  await loadDiff();
  $("diff-modal").classList.add("visible");
}

function closeDiff() {
  $("diff-modal").classList.remove("visible");
}

function updateReviewBadge() {
  const badge = $("review-badge");
  // Badge counts actionable problems only — disabled/rejected rows are
  // deliberate, not issues to flag.
  const count = state.problems ? (state.problems.actionable ?? state.problems.total) : 0;
  badge.textContent = String(count);
  badge.hidden = count === 0;
}

function wireEvents() {
  $("review-open-button").addEventListener("click", () => openReview().catch((error) => showMessage(error.message)));
  $("review-close-button").addEventListener("click", closeReview);
  $("review-scrim").addEventListener("click", closeReview);
  $("diff-open-button").addEventListener("click", () => openDiff().catch((error) => showMessage(error.message)));
  $("diff-close-button").addEventListener("click", closeDiff);
  $("error-close-button").addEventListener("click", closeBuildError);
  $("error-copy-button").addEventListener("click", async () => {
    try {
      await navigator.clipboard.writeText($("error-message").textContent || "");
      showMessage("Error copied to clipboard.");
    } catch (_e) {
      showMessage("Copy failed — select the text manually.", 6000);
    }
  });
  const debouncedSearch = () => {
    window.clearTimeout(state.searchTimer);
    state.searchTimer = window.setTimeout(() => {
      state.page = 1;
      state.gridScrollTop = 0;
      loadRows().catch((error) => showMessage(error.message));
    }, 180);
  };
  $("query-input").addEventListener("input", debouncedSearch);
  $("target-input").addEventListener("input", debouncedSearch);
  for (const f of FILTERS) f.wire();
  $("page-size").addEventListener("change", () => {
    state.pageSize = Number($("page-size").value);
    state.page = 1;
    state.gridScrollTop = 0;
    loadRows().catch((error) => showMessage(error.message));
  });
  $("runtime-filter").addEventListener("change", () => {
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
  $("duplicate-row-button").addEventListener("click", () => duplicateRow().catch((error) => showMessage(error.message)));
  $("revert-row-button").addEventListener("click", () => revertRows().catch((error) => showMessage(error.message)));
  $("soft-remove-button").addEventListener("click", () => softRemove().catch((error) => showMessage(error.message)));
  $("move-up-button").addEventListener("click", () => moveRows("up").catch((error) => showMessage(error.message)));
  $("move-down-button").addEventListener("click", () => moveRows("down").catch((error) => showMessage(error.message)));
  $("move-top-button").addEventListener("click", () => moveRows("top").catch((error) => showMessage(error.message)));
  $("move-bottom-button").addEventListener("click", () => moveRows("bottom").catch((error) => showMessage(error.message)));
  $("bulk-column").addEventListener("change", updateBulkValues);
  $("bulk-apply-button").addEventListener("click", () => bulkEdit().catch((error) => showMessage(error.message)));
  $("set-open-button").addEventListener("click", (event) => {
    event.stopPropagation();
    togglePopover("set-popover", "set-open-button");
  });
  $("overflow-open-button").addEventListener("click", (event) => {
    event.stopPropagation();
    $("overflow-popover").classList.remove("context-menu");
    $("overflow-popover").style.left = "";
    $("overflow-popover").style.top = "";
    togglePopover("overflow-popover", "overflow-open-button");
  });
  document.addEventListener("click", () => closePopovers(null));
  ["set-popover", "overflow-popover"].forEach((id) =>
    $(id).addEventListener("click", (event) => event.stopPropagation()),
  );
  $("delete-row-button").addEventListener("click", () => deleteRows().catch((error) => showMessage(error.message)));
  $("regex-open-button").addEventListener("click", openRegexModal);
  $("regex-preview-button").addEventListener("click", () => regexPreview().catch((error) => showMessage(error.message)));
  $("regex-rules-preview-button").addEventListener("click", () => regexRulesPreview().catch((error) => showMessage(error.message)));
  $("regex-apply-button").addEventListener("click", () => regexApply().catch((error) => showMessage(error.message)));
  $("regex-cancel-button").addEventListener("click", closeRegexModal);
  $("regex-rules-all").addEventListener("change", (event) => {
    for (const box of document.querySelectorAll(".regex-rule-box")) box.checked = event.target.checked;
    $("regex-apply-button").disabled = true;
  });
  ["regex-pattern", "regex-replacement", "regex-column"].forEach((id) =>
    $(id).addEventListener("input", () => {
      $("regex-apply-button").disabled = true;
    }),
  );
  $("regex-rules-list").addEventListener("change", () => {
    $("regex-apply-button").disabled = true;
  });
  $("undo-button").addEventListener("click", () => postAction("/api/undo").catch((error) => showMessage(error.message)));
  $("redo-button").addEventListener("click", () => postAction("/api/redo").catch((error) => showMessage(error.message)));
  $("save-button").addEventListener("click", saveBuildCheck);
  $("refresh-diff-button").addEventListener("click", () => loadDiff().catch((error) => showMessage(error.message)));
  $("refresh-problems-button").addEventListener("click", () => loadProblems().catch((error) => showMessage(error.message)));
  $("problem-type-filter").addEventListener("change", renderProblems);
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

  function typingTarget(event) {
    const node = event.target;
    if (!node) return false;
    if (node.isContentEditable) return true;
    const tag = node.tagName;
    return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT";
  }

  const fail = (error) => showMessage(error.message);

  document.addEventListener("keydown", (event) => {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
      event.preventDefault();
      saveBuildCheck();
    } else if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "z") {
      event.preventDefault();
      postAction("/api/undo").catch(fail);
    } else if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "y") {
      event.preventDefault();
      postAction("/api/redo").catch(fail);
    } else if (event.key === "Escape" && $("regex-modal").classList.contains("visible")) {
      closeRegexModal();
    } else if (event.key === "Escape" && $("error-modal").classList.contains("visible")) {
      closeBuildError();
    } else if (event.key === "Escape" && $("diff-modal").classList.contains("visible")) {
      closeDiff();
    } else if (event.key === "Escape" && !$("review-drawer").hidden) {
      closeReview();
    } else if (event.key === "Delete" && !typingTarget(event) && selectedOrActiveIds().length) {
      event.preventDefault();
      deleteRows().catch(fail);
    }
  });
}

async function init() {
  wireEvents();
  await loadMeta();
  makeTable();
  await loadRows();
  await loadProblems();
}

init().catch((error) => showMessage(error.message, 0));
