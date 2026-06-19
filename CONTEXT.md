# KhmerIME

A Khmer input method engine. Users type romanized Khmer (e.g. `tverdomnors`) and the IME suggests Khmer candidates (e.g. ធ្វើដំណោះ). Platform adapters expose the engine to IBus (Linux), TSF (Windows), and other host IME frameworks.

## Language

**Lexicon**:
The source-of-truth roman→Khmer dictionary, stored as CSV chunks under `data/lexicon/chunks/`. Each row is one (roman, target, frequency, classification, status) tuple.
_Avoid_: dictionary, vocabulary

**SharedTransliteratorData**:
The fully-built in-memory artifact produced from the **Lexicon** plus khPOS corpus and mobile-keyboard n-grams. Holds lookup maps, ranked lexicon, search index, and composer state. Built once per bridge process; the three view-Transliterators (live engine, visible refiner, commit refiner) are cheap clones over the same shared data.
_Avoid_: lexicon data, lookup tables

**Dictionary Image**:
A compact, immutable, file-backed representation of **Lexicon**-derived lookup data, addressed by IDs and offsets instead of heap-owned strings and maps.
_Avoid_: mmap cache, binary cache, serialized SharedTransliteratorData

**Search Index**:
The fuzzy roman→Khmer lookup structure used by the suggester. One of two backends, selected at startup by `KHMERIME_SEARCH_INDEX`: **Ngram** (default) or **SymSpell**. Both live inside the **SharedTransliteratorData**.
_Avoid_: fuzzy index, lookup index

**Composition**:
The user's current in-progress input — the raw roman string, the candidate list, the selected index, and optionally a **Segmented Session** when the decoder splits the input into multiple chunks. Resets on commit, focus-out, or escape.
_Avoid_: preedit, input buffer

**Segmented Session**:
A multi-chunk view of a long **Composition** where the decoder identifies internal word boundaries. Each segment has its own Khmer candidates and selection; the user can navigate between segments with Left/Right. In romanization mode, segmented Khmer outputs are preview choices over the raw roman **Preedit**; they do not replace the inline **Preedit** before confirmation.
_Avoid_: multi-segment preedit, split composition

**Segment Edit Mode**:
A sub-state of a **Segmented Session** in which one focused segment is being rewritten in isolation. Sibling segments are *pinned* — their Khmer outputs are locked and the decoder does not touch them. The candidate list shows candidates for the in-edit segment only, decoded as a single flat slice (no internal re-segmentation). Tab enters and exits the mode (re-pinning the segment with its currently selected candidate); Escape cancels and restores the segment's pre-edit state; Enter is unchanged and commits the whole **Composition** per [[Visible Segmented Commit]]. Digit keys 1–9 select the in-edit segment candidate without committing the whole **Composition**. Left/Right auto-exit the mode and navigate to the previous/next segment. Keystroke semantics inside the mode are asymmetric: the first printable key after entering replaces the entire roman slice (text-editor-style); Backspace deletes one char at a time (IME-style), and Backspace on an empty in-edit segment transfers Segment Edit Mode to the previous segment (or dissolves the **Segmented Session** to a flat **Composition** if no segments remain). While in Segment Edit Mode the in-edit segment's roman slice is rendered with both an underline and a background highlight in the **Preedit**; in navigate-only focus only the underline is drawn. The background highlight is an edit-mode indicator, not a literal text-editor selection.
_Avoid_: segment rewrite, active segment, segment focus

**Preedit**:
The provisional inline text displayed at the cursor while the user is composing. In romanization mode this is the user's raw roman input; Khmer choices are shown as candidates or previews until the user confirms. Distinct from the **Commit Text** — the preedit is provisional; commit text is what lands in the application.
_Avoid_: composition text (ambiguous with **Composition**), inline Khmer preview

**Candidate List**:
The visible Khmer choices for the active **Composition** or focused **Segmented Session** segment. In romanization mode, the Candidate List should remain visible even when there is only one obvious choice, because it tells the user what Enter will commit while the inline **Preedit** stays roman.
_Avoid_: suggestions (too broad), inline candidates

**Coeng Form**:
A Khmer subscript consonant used to type consonant clusters. A Coeng Form is the invisible coeng sign plus a base consonant, rendered together as a subscript shape (for example `្ក`). In **CharPick Mode**, Coeng Forms appear under the same roman letters as their base consonants so users can build clusters quickly.
_Avoid_: bare coeng sign when you mean the full subscript consonant

**Roman Hint**:
The exact romanized key or keys displayed beside a Khmer candidate to show why that candidate is available. Roman Hints are display metadata for the **Candidate List**; they do not replace the raw roman **Preedit** and do not change the **Commit Text**. If no exact Roman Hint exists for a candidate, the UI must not invent one; it should show a derived marker instead.
_Avoid_: invented hint, transliteration label, candidate subtitle

**Commit Text**:
The confirmed text sent to the host application when the user confirms. In romanization mode, Enter is the normal confirmation key; Space and digit keys select candidates or segments during an active **Composition**. Commit text is normally Khmer, selected from candidates, a **Segmented Session**, or the **Commit Refiner**; it may fall back to roman text when no Khmer output is available.
_Avoid_: output text, submission

**Visible Segmented Commit**:
The rule that when a **Segmented Session** is visible, Enter commits the currently selected segment outputs exactly as shown. The visible segmented phrase is authoritative; Enter must not replace it with a hidden refinement.
_Avoid_: WYSIWYG commit, segment preview commit

**Visible Candidate Commit**:
The rule that when the candidate list has a selected visible candidate, Enter commits that candidate exactly. Hidden refinement may fill in when no visible candidate is available, but must not replace the candidate the user can see.
_Avoid_: hidden override, invisible correction

**Hidden Commit Fallback**:
The limited use of the **Commit Refiner** when visible state is not meaningfully commit-ready. It may recover a Khmer phrase when the visible candidate is empty, raw roman, or otherwise not useful as Khmer **Commit Text**, but it must not override a visible Khmer candidate or **Segmented Session**.
_Avoid_: final refinement, commit correction

**Commit Rules**:
The ordered precedence applied at commit time to pick the **Commit Text** from a **Composition**: **Visible Segmented Commit** first, then **Visible Candidate Commit**, then **Hidden Commit Fallback**, with the raw roman string as the final floor. Each step is one of the rules above; the earliest one that yields useful Khmer wins, so visible state always outranks hidden refinement.
_Avoid_: commit resolution, commit policy, commit precedence

**Bridge**:
The Rust subprocess (`khmerime-ibus-bridge`) that owns the **SharedTransliteratorData** and serves commands from the Python IBus adapter over stdin/stdout JSON. One bridge per `KhmerIMEEngine` instance.
_Avoid_: backend, service

**Learned History**:
The per-user `HashMap<String, usize>` counting how often each Khmer **unigram** has been committed. Boosts ranking in `suggest()` and `next_word_suggestions()` alongside the static corpus statistics. Persisted via `HistoryStore` (TSV today; see ADR-0002). Keys are individual Khmer words, never concatenated multi-word phrases — when a **Commit Text** spans multiple segments (from a **Segmented Session** or the **Commit Refiner**'s WFST output), each segment is learned separately.
_Avoid_: user dictionary, learned words (ambiguous with corpus)

**English Mode**:
An input mode in which all keystrokes (letters, symbols, numbers, space, backspace, return) are routed directly to the host text field without Khmer processing or roman-buffer accumulation. Toggled by the EN key, which occupies the globe-key slot when the system keyboard switcher is not needed. English Mode is orthogonal to the visual layer — switching between QWERTY, 123, and #+= does not exit English Mode. Pressing ✦ while in English Mode exits English Mode and enters CharPick (since no Composition is active). Pressing EN while composing abandons the active Composition silently: the Rust session resets, the roman Preedit remains in the host text field as literal text, and English Mode begins.
_Avoid_: latin mode, passthrough mode, direct-input mode

**CharPick Mode**:
An input mode (`InputMode::CharPick`) for typing Khmer text that is not in the **Lexicon** — names, place names, loanwords. The user types one roman letter; the session looks up all Khmer characters and **Coeng Forms** whose phonetic relation includes that letter and returns them as the **Candidate List**. Tapping a candidate commits that single Khmer character or Coeng Form immediately to the host application with no **Composition** or preedit accumulation. Each keystroke is an independent lookup; there is no progressive multi-letter narrowing. On iOS and Android, the ✦ key toggles CharPick Mode: pressing ✦ enters it (abandoning any active **Composition** and clearing the roman buffer), pressing ✦ again exits it. While in CharPick Mode the keyboard layer remains unchanged (qwerty stays visible) and the ✦ key is visually highlighted. Letter keypresses are routed to the session without inserting text into the host field — only the **Candidate List** updates. Backspace while candidates are visible clears them and resets for a new lookup; backspace with an empty **Candidate List** deletes one character from the host text field.
_Avoid_: name mode, character picker, direct character input

**Phase A / Phase B startup**:
A two-stage warmup. Phase A builds a minimal **SharedTransliteratorData** fast enough to accept keystrokes (~100 ms). Phase B builds the full version in a background thread (~800 ms ngram, ~1300 ms SymSpell). Phase B is swapped in transparently when ready.

**Visible Refiner / Commit Refiner**:
Two distinct `Transliterator` views built from the same **SharedTransliteratorData** but with different decoder configurations. The visible refiner has a 75 ms latency budget for in-flight preview refinement; the commit refiner has a larger budget and produces the final **Commit Text** on Enter. Both budgets are wall-clock deadlines; when the commit refiner's trips, the commit degrades to the visible result rather than waiting (see ADR-0005).
_Avoid_: refiner (use the qualified form)

**Download Landing Page**:
The public KhmerIME page that helps users choose a platform download, try the **Online Beta**, and follow install steps. It is download-first, with the visitor's platform download as the primary action and the **Online Beta** as a secondary trial path.
_Avoid_: homepage, product site

**Online Beta**:
The in-browser build of KhmerIME (the dioxus-app, deployed at the beta URL) where a visitor types romanized Khmer and sees candidates without installing anything. Reached from the **Download Landing Page** as its secondary trial path; a quick trial surface, not a replacement for the installed platform IMEs.
_Avoid_: playground, demo, web app

**Silk Veil**:
The shared glassmorphic visual identity for KhmerIME's public web surfaces — the **Download Landing Page** and the **Online Beta**. A deep-ink / charcoal-plum base, soft translucent pearl-glass panes with white rim highlights, an ember-amber primary action, sparse peacock-teal accents, and warm ivory text — distinct from the previous light cream/terracotta web styling.
_Avoid_: Liquid Glass Theme, light download theme, glass accents

## Relationships

- A **Lexicon** is built into one **SharedTransliteratorData** at startup
- A **Dictionary Image** can replace heap-owned parts of **SharedTransliteratorData** while preserving the same IME behavior
- A **SharedTransliteratorData** contains exactly one **Search Index**
- A **SharedTransliteratorData** is shared by the live engine, **Visible Refiner**, and **Commit Refiner** (three views, one underlying data)
- A **Composition** may have zero or one **Segmented Session**
- A **Segmented Session** may be in **Segment Edit Mode** for at most one of its segments at a time
- A **Preedit** and **Commit Text** are derived from the same **Composition** but may diverge (see ADR-0001)
- A **Visible Segmented Commit** makes the visible **Segmented Session** authoritative over hidden commit refinement
- A **Visible Candidate Commit** makes the visible selected candidate authoritative over hidden commit refinement
- A **Hidden Commit Fallback** only applies when visible state is not useful Khmer **Commit Text**
- A **Bridge** owns exactly one **SharedTransliteratorData** for its lifetime
- The **Download Landing Page** and the **Online Beta** share the **Silk Veil** visual identity
- The **Download Landing Page** links to the **Online Beta** as its secondary trial path
