# CharPick input mode for names and un-lexiconed text

Khmer proper nouns (names, place names, loanwords) are not in the **Lexicon**, so
`InputMode::Roman` returns no useful candidates for them. Users need a way to spell
out Khmer text character by character without relying on the decoder.

## The key decisions

- **New `InputMode::CharPick` variant in the Rust session, not a Swift-side feature.**
  The lookup data (`khmer_character_relation.csv`) belongs with the engine. Every
  future platform adapter (Android, macOS) would otherwise have to re-implement the
  same lookup logic in its own UI layer. Keeping it in `crates/session` makes it
  testable in Rust without a simulator and reusable across adapters.

- **Single roman character as the lookup unit — no progressive narrowing.**
  Each keystroke independently returns all Khmer characters whose relation list
  contains that roman letter. The user types one letter, scans the candidate strip,
  and taps. Multi-character filtering (e.g., `kh` to narrow to ខ/ឃ only) was
  considered but rejected: the candidate count per single letter is small enough to
  scan on a mobile strip, and single-char lookup keeps the session API stateless
  between keystrokes.

- **Coeng Forms share their base consonant's roman relation.**
  CharPick is optimized for typing Khmer quickly, not for exposing only standalone
  letters. When a base consonant is available for a roman letter, its Coeng Form
  must also be available for the same roman letter so users can build clusters
  without hunting for a separate "choeng" mode or typing the bare coeng sign.

- **Immediate commit per candidate tap — no preedit accumulation.**
  Each tap commits one Khmer character directly to the host text field. There is
  nothing to refine or rank across multiple characters; an accumulation step would
  add a confirmation round-trip with no benefit. This matches how Nida mode commits,
  but differs in that CharPick returns a candidate list rather than a fixed keymap.

- **`khmer_character_relation.csv` bundled as `include_str!` in `crates/session`.**
  Same pattern as `nida_keymap.csv`. No runtime file I/O, no platform-specific asset
  loading, no build-time code generation required.

- **⊞ button is context-sensitive on iOS.**
  When no **Composition** is active, ⊞ enters CharPick mode (and pressing ⊞ again
  exits). When a **Composition** is active, ⊞ opens the existing candidate panel.
  This keeps ⊞ as a two-state toggle in both cases and avoids a three-stop cycle
  that would force users past an irrelevant state.

- **Backspace in CharPick mode deletes from the host text field.**
  Since there is no filter string to walk back (single-char lookup is stateless),
  ⌫ always deletes the last committed Khmer character. No special session handling
  needed beyond the default backspace path.

## What this constrains

1. `InputMode` in `crates/session/src/adapter_contract.rs` gains a `CharPick` variant.
2. `ImeSession` must handle `InputMode::CharPick` in its key-event dispatch path,
   returning a candidate list from the character-relation data rather than running
   the roman decoder.
3. CharPick candidate generation must include Coeng Forms for base consonant
   entries even when the relation CSV only lists the base consonant explicitly.
4. `IosRenderState.candidates` carries the CharPick results; `preedit` is empty in
   this mode. No new fields on `IosRenderState` are required.
5. `adapters/ios-keyboard/src/lib.rs` must expose a way for `KhmerIMESession` to
   signal the current mode so Swift can drive the ⊞ context-sensitivity correctly.
6. The `khmer_character_relation.csv` path moves (or is copied) into
   `crates/session/data/` so it can be reached by `include_str!`.

## When to revisit

- If the per-letter candidate count grows large enough that users need narrowing
  (e.g., a future expanded CSV with rare characters). Progressive filtering can be
  added without breaking the existing single-char path.
- If Android or macOS adapters implement CharPick and find the single-char model
  insufficient for their input surface (e.g., a physical keyboard where multi-char
  sequences are faster to type).
