# Plan — Phrase Wheel for Android

Mirror the iOS **Phrase Wheel** (ADR-[0014](../adr/0014-phrase-wheel-replaces-default-candidate-row.md)
/ [0015](../adr/0015-phrase-wheel-refined-after-live-testing.md)) to Android. The
engine + shared `khmerime_session` are **already done** (built for iOS): the wheel
data (`phrase_candidates` with `segments`, `selected_phrase_index`) and the
`SelectPhrase` command all live in the session and its `SessionSnapshot`. Android
work is **surfacing + UI only** — no new decoder or session logic.

## Why this is a mirror, not a rebuild

Android already has the parallels the iOS effort had to build:

| iOS | Android | Notes |
|---|---|---|
| UniFFI records | **JSON** (`serde` `RenderState` → Gson `KhmerRenderState`) | different FFI — add JSON fields, no bindgen |
| `StripView` (roman + Khmer rows) | `PreeditStripView` | already renders segments + `✏` edit marker |
| `CandidateSurfaceView` (wheel / word-candidates by mode) | `SuggestionChipView` (always-on chip row) | **repurpose** — see decision below |
| `KeyboardRootView.setChromeRows` collapse | `KeyboardPresentationSpec.chromeRows` (`None`/`CandidateOnly`/`StripAndCandidate`) | fold in "has alternatives" |
| Segment-tap → `sendTab` (Level 2) | `PreeditStripView.onSegmentFocused` → `processTab` | **already wired** |
| XCTest + fakes | JUnit + `InMemoryTextProxy` / `RecordingDispatcher` | TDD the same way |

## Resolved decision (grill)

**Q1 — repurpose `SuggestionChipView` into the mode-switching surface.** It shows
**phrase alternatives** during normal composition and **word candidates** during
CharPick / Segment Edit — Android's equivalent of iOS's `CandidateSurfaceView`. Its
contents + tap handler fork on mode; the chip infrastructure stays.

Behavior to match iOS (ADR-0015): alternatives only (**exclude `selected_phrase_index`**,
not always index 0) · **hidden when no alternatives** (strip stands alone) · **tap
selects** (previews in the strip via `select_phrase`), **Space/Enter commit** ·
centered when they fit / scroll when they overflow.

## TDD slices (order; each RED→GREEN; run `make platform-test-android`)

**1. Android FFI — surface the wheel data.**
- `adapters/android-ime/src/lib.rs`: add to `RenderState` (serde) `phrase_candidates:
  Vec<PhraseCandidateJson>` (`{ text, segments: Vec<SegmentEntry> }`) and
  `selected_phrase_index: u64`, populated from the snapshot. Add a JNI export
  `Java_..._KhmerImeSession_nativeSelectPhrase(handle, index)` calling
  `SessionCommand::SelectPhrase` (mirror `nativeProcessTab`).
- Kotlin: `KhmerRenderState` (Gson) += `phraseCandidates` (`@SerializedName("phrase_candidates")`)
  + `selectedPhraseIndex`; `KhmerImeSession.selectPhrase(index)` + `external fun nativeSelectPhrase`.
- Test (`KhmerImeSessionContractTest`): a real session typing a multi-reading input
  (`khnhom`) exposes ≥2 `phraseCandidates`, each Khmer with `segments`; `selectPhrase(i)`
  makes Enter commit reading `i`.

**2. Presentation logic.** `KeyboardPresentationSpec`:
- `phraseAlternatives(state): List<Pair<Int,String>>` = `phraseCandidates` **excluding
  `selectedPhraseIndex`** (carry the original index for tap → `selectPhrase`).
- `suggestionCandidates` forks: composition → `phraseAlternatives`; CharPick / Segment
  Edit → today's `state.candidates`.
- `chromeRows`: during composition, reserve the candidate row only when
  `phraseAlternatives` is non-empty; else `StripOnly` (see slice 5).
- Test (`KeyboardPresentationSpecTest`): exclusion of the selected index; empty when
  one reading; word candidates in CharPick/edit.

**3. Chip row UI.** `SuggestionChipView` renders the mode-appropriate list; tap in
composition → `onPhraseSelected(originalIndex)`, tap in CharPick → `onCandidateSelected`.
(`SuggestionChipView` is a canvas-drawn `View` — reuse its existing hit-testing.)

**4. Handler wiring.** `KhmerInputHandler.selectPhrase(index)` → `session.selectPhrase(index)`
→ `render` (selects; the strip previews it; **never commits**). Wire the chip row's
composition tap to it. Test (`KhmerInputHandlerBehaviorTest`): tap an alternative → strip
segments change, no commit; then Enter commits it.

**5. Hide-when-empty collapse.** Add a `ChromeRows.StripOnly` (strip visible, candidate
row height 0) and return it during composition with no alternatives. Mirror iOS's
`stripOnly` chrome state. Test in `KeyboardPresentationSpecTest`.

**Level 2 (verify, likely no code):** segment-tap → `processTab` → `segmentEditActive`
already renders `✏` + word candidates in the chip row. Confirm the chip row still shows
word candidates (not phrase alternatives) when `segmentEditActive`, per slice 2's fork.

## Files

- Rust: `adapters/android-ime/src/lib.rs`
- Kotlin main: `input/KhmerRenderState.kt`, `input/KhmerImeSession.kt`,
  `input/KhmerInputHandler.kt`, `layout/KeyboardPresentationSpec.kt`,
  `views/SuggestionChipView.kt`, `service/KhmerInputMethodService.kt` (wire the new tap callback)
- Kotlin test: `KhmerImeSessionContractTest`, `KeyboardPresentationSpecTest`,
  `KhmerInputHandlerBehaviorTest`

## Build / test / run

```
make platform-test-android      # JVM unit tests, no device (fast TDD loop)
make platform-build-android     # cargo-ndk Rust + debug APK
make platform-install-android   # onto a connected device/emulator for live check
```

## Parity gotchas

- JSON FFI: field names use `@SerializedName` snake_case to match the serde struct.
- `selected_phrase_index` defaults to `0`; `select_phrase` updates it. The chip row must
  exclude **that** index, not a hardcoded 0, or you reintroduce the iOS reversibility bug.
- Keep tap = **select**, not commit (the iOS lesson — commit-on-tap was too trigger-happy).
- Don't show raw roman as a wheel card (WFST source already prevents it engine-side).

## Done when

`make platform-test-android` green with the new slices; on device, typing a multi-word
phrase shows Khmer alternatives in the chip row, the row hides when there's one reading,
tapping an alternative previews it in the strip, and Space/Enter commits it — matching iOS.
