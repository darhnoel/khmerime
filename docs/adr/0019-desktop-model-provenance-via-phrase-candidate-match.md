# Desktop adapters derive model provenance by matching displayed candidates against phrase candidates, not by widening the shared candidate type

Desktop input-method adapters (macOS IMK, and by precedent Linux IBus and Windows TSF)
render a **Candidate List** — a vertical list of Khmer candidate strings — not the mobile
**Phrase Wheel**. To honour ADR-0016 (a model-assisted candidate must carry a visible `✦`
marker, red when unverified), each adapter needs per-candidate `from_model` / `lexicon_verified`
provenance on the rows it paints.

The obvious way would be to add those two fields to the shared `CandidateDisplayEntry` in
`crates/session` — the type every adapter maps its candidate rows from. We deliberately do
**not** do that.

## Decision

The shared `CandidateDisplayEntry` stays narrow (no provenance fields). Instead a desktop
adapter derives provenance **on its own side** by matching each candidate it displays against
`SessionSnapshot.phrase_candidates`, which already carry `from_model` and `lexicon_verified`.

Concretely, in the adapter's snapshot → render-state mapping:

1. Build a lookup from `snapshot.phrase_candidates`, keyed by
   `normalized_suggestion_key(candidate.text)` → `(from_model, lexicon_verified)`.
2. When mapping each displayed candidate (`candidate_display[i].output`), look up
   `normalized_suggestion_key(output)` in that map and stamp the two flags onto the adapter's
   **own** candidate type (e.g. `MacosCandidateDisplayEntry`), defaulting to
   `(false, true)` — a plain Lexicon candidate — when absent.

The match is safe and cheap because the provider is a **Word Rescuer** (see CONTEXT.md): it
contributes at most one whole-**Composition**-span word per refinement, so at most one row is
ever `from_model`, and `normalized_suggestion_key` is the same helper the commit rules already
use to compare candidates.

## Why not widen `CandidateDisplayEntry`

- **It is a shared contract touched by every adapter** (mobile and desktop). Widening it to
  serve the desktop marker forces a field onto types and code paths that already express
  provenance a different way (mobile uses `PhraseCandidate.from_model` on the Phrase Wheel).
- **The provenance already exists in the snapshot**, on `phrase_candidates`. Nothing needs to
  be recomputed or newly threaded through the session — the desktop adapter just reads a field
  it was ignoring. Adding a second carrier of the same truth invites the two drifting apart.
- Keeping the change **inside the adapter** matches the repo's boundary discipline: an
  adapter-specific rendering concern is solved in the adapter, not by reshaping the core
  contract.

## The marker is load-bearing UI — white vs red

This ADR exists to *feed* the ADR-0016 marker on desktop; the marker's meaning is unchanged and
must not regress:

- **No ✦** — a plain Lexicon / fuzzy candidate (`from_model == false`). Human-reviewed data.
- **White ✦** — `from_model && lexicon_verified`: the model rescued this word, **and** it is a
  real Lexicon word. Model-assisted but trusted.
- **Red ✦** — `from_model && !lexicon_verified`: the model produced Khmer that is **not** in the
  Lexicon. Shown, never hidden, but visibly unverified so it can never masquerade as
  human-reviewed data.

Red candidates are **kept visible on purpose.** An out-of-Lexicon model word is not necessarily
wrong: it may be a valid **name or loanword** the user genuinely wants that simply is not in the
Lexicon yet. Per ADR-0016 the Lexicon gate is a *marker, not a filter* — the user decides. The
adapter must therefore render red-marked candidates as selectable, not drop them.

## Ranking and count

- The adapter shows the top **3–5** candidates (the panel's page size).
- Ranking is the engine's existing order: by frequency and Lexicon membership, so a trusted
  **Lexicon** candidate always ranks ahead of an unverified (red) one (ADR-0016). The adapter
  does not re-rank; it renders the engine's ranked list and only adds the provenance marker.

## Consequences

- Each desktop adapter adds `from_model` / `lexicon_verified` to its **own** candidate record
  and a `normalized_suggestion_key` match in its render mapping; the shared session contract is
  untouched.
- The `✦` marker + red colour must be implemented in each desktop candidate UI (macOS
  `CandidatePanel`, etc.), mirroring the mobile Phrase Wheel / Strip.
- If a future provider ever segments (contributes more than one span), the per-row match still
  holds — each model-sourced row matches its own phrase candidate — but the "at most one marked
  row" simplification no longer applies; revisit the page-level presentation then.

## See also

- [ADR-0016](0016-runtime-model-provider-behind-a-lexicon-verified-marker.md) — the marker,
  not-a-filter principle, and why unverified output reaches the user.
- [ADR-0015](0015-phrase-wheel-refined-after-live-testing.md) — the Phrase Wheel reads the WFST
  decoder, so the top phrase candidate equals the segmented preview; the invariant that makes the
  match reliable.
- CONTEXT.md — **Word Rescuer**, **Standard / Smart Mode** (macOS: Smart is implicit when a
  provider is armed).
