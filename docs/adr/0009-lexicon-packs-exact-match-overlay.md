# ADR-0009: Lexicon Packs as exact-match overlays

**Status:** Accepted

## Context

Two user-facing wishes turned out to be the same shape:

- **User custom words** — letting users add their own roman→Khmer entries. This already
  half-exists, but only in the web app: `merge_with_user_dictionary` in
  `apps/dioxus-app/src/ui/editor/candidate_pipeline.rs` prepends exact-match user entries
  into the candidate list *inside the app*, so the native adapters (macOS IMK, iOS, Windows
  TSF, Linux IBus) never see them. This violates the repo principle that input behavior
  lives in the shared engine crates.
- **"Code-switching in other languages"** — on inspection this is not linguistic
  code-switching. The intent is pluggable secondary word sets (tech, medical, loanword
  packs) layered on the base **Lexicon**, still producing Khmer output.

Both are overlays of explicit roman→Khmer entries on top of the base **Lexicon**, much like
**Learned History** is an overlay that re-ranks without being baked into
**SharedTransliteratorData**.

## Decision

Introduce one concept, the **Lexicon Pack**, covering both cases:

- **One mechanism, two kinds.** An always-on, editable *personal pack* (the user's own
  words) and read-only *curated packs* the user toggles on. Same schema, same lookup path.
- **Exact-match only.** Packs are consulted as a side map keyed by the normalized roman
  token; pack entries surface only on an exact key match. Packs do **not** enter the fuzzy
  **Search Index**, so enabling/editing a pack never rebuilds **SharedTransliteratorData**.
  (Promoting packs into the fuzzy index is a possible future change, deliberately deferred.)
- **Precedence.** Personal pack first, then enabled curated packs in user-defined order,
  then base **Lexicon**. **Learned History** stays a cross-cutting ranking boost, not its
  own tier. This preserves today's "my word shows first" behavior.
- **Applied in the engine/session layer**, not per adapter, so every surface inherits
  identical behavior. The app-only `user_dictionary` merge is removed in favor of this.
- **Stable ID + version per pack** so a future remote pack registry can deliver and update
  packs without a format change.

## Consequences

- The web app's `user_dictionary` localStorage data becomes the *personal pack*; the merge
  logic moves down into the engine and the app stops doing candidate merging itself.
- Native adapters gain custom words and curated packs for free once they read the
  **Config Store** (see ADR-0010).
- Exact-match means custom words require precise typing — no typo tolerance — until/unless
  the fuzzy-index seam is taken. This is an accepted v1 limitation, matching current
  `user_dictionary` behavior.
- "Code-switching" is intentionally *not* implemented as alternating languages; output
  stays Khmer. The term is on the _Avoid_ list for **Lexicon Pack** in CONTEXT.md.
