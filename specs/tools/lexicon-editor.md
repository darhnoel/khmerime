# Lexicon Editor Tool

This spec defines the MVP contract for a local maintenance tool that edits
`data/lexicon/chunks/*.csv` and rebuilds `data/roman_lookup.csv`.

The tool is for trusted repository maintainers. It is not a user-facing IME
feature and must not be mixed into the Dioxus app.

## Goals

- Provide a local spreadsheet-style editor for lexicon chunk CSV rows.
- Keep chunk files as the reviewed source of truth.
- Reuse the existing chunk validation and runtime CSV generation logic from
  `scripts/data/lexicon/manage_lexicon_chunks.py`.
- Make edits through server-side drafts first, then write files only when the
  user explicitly saves.
- Run the build/check flow and show `git diff`; do not create Git commits.

## Non-Goals

- Do not change the runtime `data/roman_lookup.csv` schema.
- Do not add authentication, network sharing, or multi-user collaboration.
- Do not add decoder suggestion testing in the MVP.
- Do not use the main Dioxus app for maintenance-only filesystem access.
- Do not add a generic file browser or arbitrary filesystem API.

Runtime output remains generated from approved chunk rows only:

```text
roman,target,freq,freq_lang
```

Chunk-only fields remain review metadata:

```text
category,status,notes
```

## Stack

- Python standard-library local server.
- Vendored Tabulator for the editable spreadsheet grid.
- Small static JavaScript controller for API calls and page state.
- No npm build step.
- No runtime CDN dependency.

Expected layout:

```text
tools/lexicon-editor/
  server.py
  static/
    index.html
    app.js
    style.css
    vendor/tabulator/
      tabulator.min.js
      tabulator.min.css
      README.md
```

Expose the tool with:

```text
make lexicon-editor
```

The command prints the local URL. It should not open a browser automatically.

## Data Model

Chunk rows use the existing columns from `manage_lexicon_chunks.py`:

```text
roman,target,freq,freq_lang,category,status,notes
```

Allowed values must come from the shared chunk manager:

```text
freq_lang: km, en, ja, zh, ko
category: unclassified, words, names, places, phrases, common
status: approved, draft, rejected, disabled
```

The editor must import or refactor shared Python helpers from
`manage_lexicon_chunks.py` instead of duplicating validation, generated runtime
text, or allowed-value definitions.

## Session And Drafts

The server is single-session:

- one server process owns one draft state;
- multiple browser tabs may connect, but they share that state;
- no per-user sessions are required.

Drafts are lazy and chunk-level:

- startup scans chunk names and file metadata only;
- search reads clean chunks from disk as needed;
- when a row in a chunk is edited, that chunk is loaded into a mutable draft;
- dirty search/filter results use draft rows for dirty chunks and disk rows for
  clean chunks;
- save writes only dirty chunks.

Row IDs must be stable within the running server session:

- existing rows get IDs derived from chunk filename plus original load sequence
  and original row content;
- new rows get temporary `new:<counter>` IDs;
- row movement reorders row objects without changing IDs;
- IDs may be regenerated after a successful save/reload.

The UI should show both:

- current row position in the draft/source chunk;
- original CSV line number when available.

## Safety Rules

Before writing any chunk:

- check that touched chunk files have not changed on disk since the draft was
  loaded;
- refuse to save and ask for guarded reload if an external change is detected;
- create timestamped backups for dirty chunks under `.lexicon-editor/backups/`;
- write through a temporary file, validate/read it back, then atomically replace
  the chunk file.

The app must provide guarded reload:

- if no dirty draft exists, reload from disk immediately;
- if dirty draft exists, require confirmation that unsaved changes will be
  discarded.

## UI Contract

The main editor is a paged Tabulator grid.

Required controls:

- global search across `roman`, `target`, and `notes`;
- chunk filter;
- status filter;
- category filter;
- page size selector;
- add row;
- soft remove selected rows;
- row movement controls;
- safe enum bulk edits;
- undo/redo;
- save/build/check;
- diff view;
- guarded reload.

Default pagination:

```text
page size: 100
options: 50, 100, 250
```

Server-side pagination is required. Search and filters are applied on the server
before paging. Unsaved draft changes must be reflected in search/filter results.

## Editing Behavior

All columns are editable:

- `roman`: required text
- `target`: required text
- `freq`: positive integer
- `freq_lang`: dropdown
- `category`: dropdown
- `status`: dropdown
- `notes`: text

Cell changes update the server draft immediately. They do not write disk files.

Validation happens immediately per cell and per row:

- invalid cells are highlighted;
- invalid rows may remain in draft while editing;
- save/build/check is blocked while invalid rows exist.

Warnings should be shown for review risks such as duplicate approved runtime keys
or inconsistent frequencies for the same target/frequency language. Existing
compiler/check failures remain authoritative.

The grid should include an `runtime` indicator:

```text
status=approved  -> included
status!=approved -> excluded
```

## Add, Remove, Move

Add row:

- inserts below the currently selected row;
- if no row is selected, inserts at the top of the current page/context;
- if the current filtered/search context has no single chunk, require selecting
  a target chunk;
- default values are:

```text
freq=1
freq_lang=km
category=unclassified
status=approved
notes=
```

Soft remove:

- selected rows are set to `status=disabled`;
- append an audit note like `disabled in lexicon editor YYYY-MM-DD`;
- rows already `disabled` are unchanged;
- rows already `rejected` remain rejected.

Row movement:

- support move up, move down, move to top of chunk, move to bottom of chunk;
- movement is same-chunk only;
- movement is disabled while text search or problem filters are active, because
  filtered order is ambiguous;
- runtime `data/roman_lookup.csv` order remains compiler-owned.

Bulk edits:

- allow selected rows to set `category`;
- allow selected rows to set `status`;
- allow selected rows to set `freq_lang`;
- do not include bulk text replacement or regex transforms in the MVP.

## Undo And Redo

Maintain a draft undo/redo stack for:

- cell edit;
- add row;
- soft remove;
- row movement;
- safe enum bulk edit;
- revert row.

Successful save/build/check reloads written chunks and clears dirty state plus
undo/redo history.

## Problem Queue

The MVP includes a problem queue that lists actionable row groups:

- duplicate approved runtime key: same `roman`, `target`, and `freq_lang`;
- same `target` and `freq_lang` with inconsistent `freq`;
- missing required field;
- invalid enum/value;
- disabled/rejected rows for review.

Roman spellings that map to multiple Khmer targets and Khmer targets that have
many roman aliases are normal lexicon behavior. They should be handled through
search/visualization workflows, not shown as MVP problem warnings.

The problem queue should open affected rows in the grid and allow normal edit,
soft remove, and safe enum bulk actions. It must not perform automatic destructive
cleanup.

## Save, Build, Check, Diff

The primary action is:

```text
Save -> build runtime CSV -> check -> show git diff
```

Behavior:

1. Block if draft rows have validation errors.
2. Verify dirty chunk files have not changed externally.
3. Backup dirty chunk files.
4. Write dirty chunks.
5. Generate `data/roman_lookup.csv` using shared chunk manager logic.
6. Run the same consistency check as `manage_lexicon_chunks.py check`.
7. On full success, reload touched chunks from disk, clear dirty state, clear
   undo/redo history, and show `git diff`.
8. On failure, show the error and keep the draft available for correction.

The editor must not run `git commit`.

The diff view is scoped to:

```text
data/lexicon/chunks/
data/roman_lookup.csv
```

## API Surface

The local server exposes a narrow JSON API. It must not expose generic file read
or write operations.

Required endpoints:

```text
GET  /api/meta
GET  /api/rows?query=&chunk=&status=&category=&page=&page_size=
POST /api/edit-cell
POST /api/add-row
POST /api/soft-remove
POST /api/move-rows
POST /api/bulk-edit
POST /api/undo
POST /api/redo
POST /api/revert-row
POST /api/discard-draft
POST /api/reload
GET  /api/problems
POST /api/save-build-check
GET  /api/diff
```

## Keyboard Shortcuts

Support basic spreadsheet shortcuts:

```text
Ctrl+S   save/build/check
Esc      cancel current cell edit or close modal
Enter    commit current cell and move down
Tab      commit current cell and move right
Ctrl+Z   undo
Ctrl+Y   redo
Delete   soft-remove selected rows after confirmation
```

## Verification

MVP verification is focused on the tool:

```text
python3 tools/lexicon-editor/server.py --check
python3 scripts/data/lexicon/manage_lexicon_chunks.py check
```

Manual browser smoke:

- open the printed localhost URL;
- search globally;
- filter by chunk/status/category;
- edit a cell and see draft dirty state;
- add a row near the selected row;
- soft remove selected rows;
- move rows within the same chunk;
- perform safe enum bulk edit;
- undo/redo draft operations;
- inspect problem queue;
- save/build/check;
- view git diff;
- guarded reload with and without dirty changes.

Run Rust or browser IME tests only when lexicon changes intentionally need
decoder/UI confidence beyond the editor workflow.
