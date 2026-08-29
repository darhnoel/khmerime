# Redirect: the Flick keyboard is an add-a-pair Khmer input, NOT a document-typing mode

## TL;DR
The uncommitted `FlickKeyboard` you built types into the **document textarea** as a
top-level input mode. That is the wrong interpretation of what this feature is for.
**Confirmed with the user (native speaker, product owner):** the Flick keyboard exists
only to enter the **Khmer half of a `roman → khmer` pair** that gets saved into
`user_dictionary`. It is the input method for a form field — it must never write into
the Document.

Keep your gesture/preview work — retarget where its output goes.

## Why (the actual product reason)
KhmerIME's transliterator fails on **out-of-dictionary words** — names, loanwords, etc.
`khnhom → ខ្ញុំ` works; a personal name the lexicon never saw does not. The escape hatch
is the **user dictionary**: the user teaches a pairing (their romanization → the exact
Khmer), and normal-mode ranking then decodes it. Typing Roman is exactly what's failing,
so the user needs to place the correct Khmer **directly** — that is the Flick keyboard's
one job. It produces the Khmer value for a saved pair. Nothing else.

## Correct design (agreed, with ASCII the user approved)
- Sidebar **Saved words** section stays a clean scrollable list (roman → khmer, delete ×).
- A **"+ បន្ថែម"** (add) button opens a **modal**: an add-a-pair form.
- The modal has:
  - `អក្សរឡាតាំង` (roman) — a plain `<input>`, ASCII typing.
  - `អក្សរខ្មែរ` (khmer) — a field whose value is the Flick **Preedit**, filled ONLY by the
    Flick keyboard (tap = center, lean = family member; ⌫ pops one Entry Unit; ដកឃ្លា settles).
  - The **Flick keyboard** grid (your component, retargeted).
  - `រក្សាទុក` (save) / `បោះបង់` (cancel).
- Save builds `ManualSaveRequest { roman, khmer: preedit.text() }` and calls the EXISTING
  `save_manual_save_request` (or a new `save_pair(roman, khmer, state)` — see below).
- Modal closes; the pair appears in the Saved-words list; normal-mode decoding improves.

Confirmed Khmer labels (do not guess/change): add = `បន្ថែម`; add word title = `បន្ថែមពាក្យ`;
my words section = `ពាក្យរបស់ខ្ញុំ` (or keep existing `ពាក្យរក្សាទុក`); roman = `អក្សរឡាតាំង`;
khmer = `អក្សរខ្មែរ`; save = `រក្សាទុក`; cancel = `បោះបង់`.

## Concrete refit steps
1. **Retarget `FlickKeyboard`** (`apps/dioxus-app/src/ui/components/flick_keyboard.rs`):
   - Delete the document path: `apply_document_edit`, `current_editor_selection`,
     `save_editor_text`, `insert_at`/`backspace_at` against the Document, `pending_caret`.
   - The component takes a `preedit: Signal<Preedit>` (the modal owns it). Tap/lean →
     `preedit.push(member)`; ⌫ → `preedit.backspace()`; ដកឃ្លា/↵ do NOT apply here (the
     modal decides — likely ដកឃ្លា settles/space is irrelevant for a single word; drop ↵).
   - The `Preedit` core in `editor/flick.rs` already has push/backspace/text/take/is_empty.
     `DocEdit`/`insert_at`/`backspace_at` are now UNUSED by this feature — leave them or
     remove if nothing else uses them (grep first).
2. **Build the add-pair modal** (new component, e.g. `add_pair.rs`), mounted from the
   toolbar, NOT from `editor_card.rs`. Remove the `if is_flick { FlickKeyboard { state } }`
   render at `editor_card.rs:520`.
3. **Toolbar** (`toolbar.rs`): the "+ បន្ថែម" button lives in the `ពាក្យរក្សាទុក` section and
   opens the modal (a `use_signal(|| false)`). Remove the top-level
   `ManualCharacterTyping` input-MODE button (lines ~115-120) — Flick is not a mode.
   The existing `manual_save_request` banner (lines 181-196) can go — the modal saves
   directly.
4. **state.rs**: drop `InputMode::ManualCharacterTyping` (leave only `NormalWordSuggestion`),
   its `.label()` arm, and the whole Roman-builder `ManualTypingState` /
   `ManualTypingCheckpoint`. **Keep** `ManualSaveRequest { roman, khmer }` and
   `user_dictionary`. Consider renaming `manual_*` → `pair_*` for clarity.
5. **Delete the Roman-assisted builder** in `manual_flow.rs`: `manual_filtered_candidates`,
   `apply_manual_candidates`, `refresh_manual_state_candidates`, `set_manual_kind_filter`,
   `skip_manual_roman_char`, `select_manual_candidate`, `finalize_manual_selection`,
   `commit_manual_selection`, `undo_manual_step`, and the `ManualComposeKind`/
   `ManualComposeCandidate` plumbing threading through `candidate_pipeline.rs` and
   `editor_card.rs` (the Consonant/Vowel/Subscript counts + tabs at ~editor_card.rs:165-200).
   **Keep** `save_manual_save_request`, `remove_user_dictionary_mapping`,
   `normalize_user_dictionary_key`.
6. **ADR-0002** (`docs/adr/0002-direct-khmer-entry-uses-an-embedded-flick-keyboard.md`):
   rewrite the decision — Flick is the Khmer input method for the add-a-pair dictionary
   flow, not a direct-document-entry mode. (Its current framing matches the wrong design.)
7. **Tests** (`tests/test_web_ui.py`): the browser tests should open the add-pair modal,
   flick a Khmer value, type a roman key, save, and assert the pair appears in the
   Saved-words list AND improves a subsequent normal-mode decode — NOT that Flick inserts
   into the Document.

## Keep (do not touch)
- `editor/flick.rs` pure core — KEYMAP (matches latest `keymap.js` on
  `experiments/flick-keyboard`, verified byte-identical), `resolve` (matches `directionOf`,
  7px/dominant-axis/empty→center), `Preedit`. 16 tests green.
- `user_dictionary` store, Saved-words list, normal-mode ranking consumption.
- `save_manual_save_request` / `remove_user_dictionary_mapping` / `normalize_user_dictionary_key`.

## Guardrails
- Khmer has no visible word spaces; a saved khmer value is one connected word.
- Do not replace the textarea with contenteditable.
- Do not commit `experiments/`.
- Confirm any NEW Khmer UI wording with the user before shipping.
- The user owns the product decision here: Flick = add-pair Khmer input, full stop.

## Source of truth for the layout/interaction
`experiments/flick-keyboard` branch → `experiments/khmer-flick-keyboard/` (keymap.js,
keyboard.js). Read-only via `git show origin/experiments/flick-keyboard:<path>`.
