# ADR-0010: Shared cross-platform Config Store

**Status:** Accepted

## Context

User settings (custom words, enabled packs, next-word suggestion behavior) need to be
authored once and take effect in every KhmerIME surface. Today there is no shared config:

- Web (dioxus-app) uses origin-sandboxed browser `localStorage`.
- Desktop/Linux/Windows persist **Learned History** as a TSV under `~/.config/khmerime/`
  via `HistoryStore`, but that logic lives inside the `khmerime_linux_ibus` adapter — the
  Dioxus app even depends on that Linux adapter just to load history.
- macOS IMK and iOS keyboard extensions run sandboxed inside other apps; they cannot read
  `~/.config` and have no shared location with a settings GUI.

A separate settings *app* can only configure other apps if there is a shared, agreed store
each adapter reads at runtime. The hard part is reachability across sandboxes, not the UI.

## Decision

- **Format:** a human-readable `config.toml` (next-word on/off, count shown,
  learn-from-typing flag, ordered list of enabled **Lexicon Pack** IDs) plus one
  `roman\tKhmer` TSV per pack. TOML matches `config/data_paths.toml`; TSV matches the
  history precedent that avoids CSV quoting for Khmer. SQLite was rejected — it would add a
  DB dependency to every adapter for data read once at startup.
- **Ownership:** a new `khmerime_config` crate owns the schema, load/save, and the
  platform path logic. `crates/session`/`crates/core` depend on it to apply config; every
  adapter and the settings app use it. This removes the "history lives in linux-ibus" smell.
- **Reachability:**
  - Desktop/Linux/Windows → XDG config dir (`~/.config/khmerime/`), alongside history.
  - macOS IMK + iOS keyboard → a shared **App Group** container (new entitlements).
  - Web → **not** included; browser storage stays islanded. Cross-device sync, if ever
    wanted, is a separate server/account concern.
- **Applied by the engine**, so behavior is identical across adapters.
- **Settings UI is per-platform, not one Dioxus app.** A Dioxus settings app serves
  Desktop/Linux/Windows. macOS gets a Swift IME preferences window; iOS adds a settings
  screen to the existing Swift host app. All read/write the same store. The unification is
  the store, not the UI — chosen to leverage the existing Swift apps and avoid shipping a
  second iOS bundle / immature Dioxus-on-iOS.

## Consequences

- New App Group entitlements + provisioning are required on macOS and iOS; this is
  migration-expensive to change once users have data, hence recording it here.
- The Dioxus app's dependency on `khmerime_linux_ibus` for history can be retired in favor
  of `khmerime_config`.
- Distribution of curated packs starts as **bundled + user-imported**; a downloaded
  **registry** is a deferred phase, unblocked by the stable pack ID/version from ADR-0009.
- Three settings UIs to maintain instead of one, accepted as the cost of staying idiomatic
  per platform.
