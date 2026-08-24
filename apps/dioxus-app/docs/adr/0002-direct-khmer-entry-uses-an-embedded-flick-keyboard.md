# ADR-0002: Flick input belongs to the add-pair dictionary flow

## Context

KhmerIME transliterates known Roman spellings well, but names, loanwords, and
other out-of-dictionary words need a user-defined `roman → khmer` mapping. The
user can type the Roman key normally; the hard part is supplying the exact Khmer
value when transliteration does not already know it.

An earlier Manual Character Typing flow tried to assemble Khmer from Roman input
inside the Document. A later prototype mounted a Flick keyboard as another
top-level document-entry mode. Both interpretations put the escape hatch in the
wrong place: they competed with normal transliteration and let Flick gestures
mutate document text.

## Decision

- Flick is the Khmer input method for the **add-a-pair user-dictionary modal**.
  It is not a document input mode.
- The Saved words section exposes `+ បន្ថែម`. It opens a modal containing:
  `អក្សរឡាតាំង`, `អក្សរខ្មែរ`, the Flick grid, `រក្សាទុក`, and `បោះបង់`.
- The modal owns a `Preedit`. A tap pushes the center member; a directional lean
  pushes that family member; backspace removes one Entry Unit. Multi-code-point
  members such as `ឲ្យ` therefore disappear atomically.
- The Khmer field is read-only to ordinary text input and displays only
  `preedit.text()`. Flick never reads or writes the Document textarea, editor
  selection, candidate state, or caret.
- Saved values are single connected Khmer words, so the embedded keyboard does
  not expose Space or Return.
- Save constructs `ManualSaveRequest { roman, khmer }`, persists it through the
  existing user-dictionary boundary, closes the modal, and refreshes the Saved
  words list. Normal-mode ranking consumes the mapping on subsequent decoding.
- The frequency-oriented 4×5 keymap, gesture resolver, and Entry Unit stack stay
  pure and independently tested. Browser coverage proves the modal boundary and
  the saved mapping's effect on normal decoding.

## Consequences

- The Document has one input system: normal Roman transliteration and its normal
  suggestion pipeline.
- Out-of-dictionary correction becomes explicit and teachable without requiring
  the transliterator to solve the word first.
- Temporary add-pair state disappears when the modal closes; persistent editor
  state contains only the saved user dictionary.
- The obsolete Manual builder, category tabs, Skip/Undo flow, save banner, and
  top-level Flick mode are removed.
