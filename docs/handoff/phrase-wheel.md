# Handoff — Phrase Wheel (mobile candidate UX)

Pick-up doc for continuing the **Phrase Wheel** work. Read the two ADRs first, they
are the spec: [ADR-0014](../adr/0014-phrase-wheel-replaces-default-candidate-row.md)
(original model) and [ADR-0015](../adr/0015-phrase-wheel-refined-after-live-testing.md)
(refinements from live testing — supersedes parts of 0014). Domain terms are in
[CONTEXT.md](../../CONTEXT.md): **Phrase Candidate**, **Phrase Wheel**, **Phrase Edit**.

Everything below is committed on `dev` and green (Rust + Swift). Nothing is on `main`.

## What it is

Typing romanized Khmer decodes to whole-phrase Khmer hypotheses. The **strip**'s
Khmer Row shows the top reading; the **Phrase Wheel** (the candidate-row slot) shows
the *alternative* readings. On iOS this is live; Android is not started.

## Data flow (where to look)

```
decoder (WFST / weighted-span)                 crates/core/src/decoder/
  DecoderManager::phrase_candidates()          manager.rs   ← ranked whole-phrase list (ADR-0015: WFST, not legacy)
  → Transliterator::phrase_candidates()        roman_lookup/transliterator.rs
SESSION                                         crates/session/src/
  SessionSnapshot.phrase_candidates            adapter_contract.rs   (PhraseCandidate { text, segments })
  populated in recompute_composition_state     segmented_session.rs
  SessionCommand::SelectPhrase(i)              ime_session.rs → segmented_session.rs::select_phrase
    rebuilds segmented_session from finals[i]   (so Space/Enter commit that reading)
iOS FFI (UniFFI)                                adapters/ios-keyboard/src/lib.rs
  IosRenderState.phrase_candidates             (IosPhraseCandidate { text, segments })
  KhmerIMESession::select_phrase(index)
SWIFT UI                                        adapters/ios-keyboard/swift/KhmerIMEKeyboard/Views/
  CandidateSurfaceView   hosts wheel (composition) + CandidateRowView (CharPick) + word candidates (Segment Edit)
  PhraseWheelView        alternatives only (exclude selected_phrase_index); centered/scroll via CandidateRowLayout; tap → onPhraseSelected
  PhraseWheelLayout      pure snap math (mostly unused now that tap replaced snap — candidate for deletion)
  handler.selectPhrase(at:)  Input/KeyboardInputHandler.swift  (tap → select_phrase → re-render; NEVER commits)
```

Behavior now (ADR-0015): wheel = **alternatives only**, **hidden when none**,
**centered when they fit / left-pad + scroll when they overflow**, **tap = select**
(previews in the strip), **Space/Enter = commit**.

## Build & test

```
# Rust
cargo test -p khmerime_core -p khmerime_session -p khmerime_ios_keyboard

# Swift (needs the xcframework built once via `make platform-build-ios`, and a booted sim)
xcodegen generate --spec adapters/ios-keyboard/swift/project.yml --project adapters/ios-keyboard/swift
xcodebuild test -project adapters/ios-keyboard/swift/KhmerIME.xcodeproj -scheme KhmerIMEKeyboardTests \
  -destination 'platform=iOS Simulator,id=<iPhone sim UDID>'

# After a Rust FFI change: regenerate bindings + xcframework
make platform-build-ios
```

**Gotchas:**
- The `KhmerIMEKeyboardTests` target lists shared `KhmerIMEKeyboard/**` sources
  **explicitly** in `project.yml` (it does not link the extension). Adding a new
  Swift file under `Views/`/`Input/`/… means adding a `- path:` line there, then
  `xcodegen generate`. Otherwise you get "cannot find X" / undefined-symbol linker errors.
- `Generated/*.swift` + `Frameworks/*.xcframework` are gitignored build artifacts.
- Tests use real Khmer inputs (`khnhom`, `komtovna`) — do not reintroduce placeholder
  romanizations like `foo`.

## Open items (priority order)

1. **Live verification.** The whole feature was built on unit tests + one render
   snapshot; it has **not** been driven in the running keyboard end-to-end. Do a
   `make platform-install-ios-sim`, enable the keyboard, type a multi-word phrase, and
   confirm: Khmer cards (not roman), wheel hidden when one reading, centered/padded
   layout, tap-selects-then-Enter-commits.

2. **Level 2 polish.** Word editing already works (strip chip tap → `sendTab` → Segment
   Edit; `CandidateSurfaceView` shows the word candidates when `segmentEditActive`). The
   grilled niceties are unbuilt: double-touch-on-a-wheel-card to enter edit, and
   *staying* in edit after picking a word (it currently exits). See ADR-0014's Level 2 section.

3. **Android mirror.** Nothing done. Mirror the FFI record + a Kotlin wheel on Android's
   always-on candidate row + `select_phrase`.

4. **Deferred engine work.** "Keep both same-Khmer/different-segmentation" (relax the
   decoder's text-dedup — golden-guarded) and the single-decode-per-keystroke perf
   cleanup (`phrase_candidates` currently decodes in addition to `suggest`/`shadow_observation`).

## Resolved after this handoff

- **Tap-select reversibility.** The snapshot/FFI now exposes `selected_phrase_index`
  (default `0`, updated by `select_phrase`), and the iOS wheel excludes the selected
  phrase index instead of always dropping index `0`. Selecting an alternative makes the
  original best visible again, and tapping it restores the strip preview.
- **Hide-when-empty height collapse.** iOS chrome now has a `stripOnly` state. During
  normal composition, the candidate-row height is reserved only when the Phrase Wheel
  has at least one visible alternative after excluding `selected_phrase_index`; otherwise
  the strip remains visible and the candidate row collapses to zero height.
- **Segmented phrase alternatives.** `phrase_candidates` now keeps WFST as the primary
  source and appends segmented legacy/composer phrase combinations, filtered to Khmer
  whole-phrase candidates. This restores wheel alternatives for inputs such as
  `nhomttovsalarien` (`ខ្ញុំទៅសាលារៀន`, `ខ្ញុំទៅ៏សាលារៀន`, ...), without reintroducing
  raw roman wheel cards.

## Commit trail (on `dev`)

`c4bf3fd` surface ranked list → `1f03f38` SelectPhrase + first view → `bdc1ffe`
scroll-snap → `aa07736` CandidateSurfaceView (CharPick fix) → `891fc64` Level-2
rendering → `4ffdcf6` WFST source + ADR-0015 → `cddf3d7` alternatives/balanced/tap-commit
→ `d41ea70` tap-selects-not-commits.
