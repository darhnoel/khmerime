# KhmerIME

A Khmer input method engine. Users type romanized Khmer (e.g. `tverdomnors`) and the IME suggests Khmer candidates (e.g. ធ្វើដំណោះ). Platform adapters expose the engine to IBus (Linux), TSF (Windows), and other host IME frameworks.

## Language

**Lexicon**:
The source-of-truth roman→Khmer dictionary, stored as CSV chunks under `data/lexicon/chunks/`. Each row is one (roman, target, frequency, classification, status) tuple.
_Avoid_: dictionary, vocabulary

**SharedTransliteratorData**:
The fully-built in-memory artifact produced from the **Lexicon** plus khPOS corpus and mobile-keyboard n-grams. Holds lookup maps, ranked lexicon, search index, and composer state. Built once per bridge process; the three view-Transliterators (live engine, visible refiner, commit refiner) are cheap clones over the same shared data.
_Avoid_: lexicon data, lookup tables

**Search Index**:
The fuzzy roman→Khmer lookup structure used by the suggester. One of two backends, selected at startup by `KHMERIME_SEARCH_INDEX`: **Ngram** (default) or **SymSpell**. Both live inside the **SharedTransliteratorData**.
_Avoid_: fuzzy index, lookup index

**Composition**:
The user's current in-progress input — the raw roman string, the candidate list, the selected index, and optionally a **Segmented Session** when the decoder splits the input into multiple chunks. Resets on commit, focus-out, or escape.
_Avoid_: preedit, input buffer

**Segmented Session**:
A multi-chunk view of a long **Composition** where the decoder identifies internal word boundaries. Each segment has its own candidates and selection; the user can navigate between segments with Left/Right.
_Avoid_: multi-segment preedit, split composition

**Preedit**:
The Khmer text displayed inline at the cursor while the user is composing. Distinct from the **Commit Text** — the preedit is provisional; commit text is what lands in the application.
_Avoid_: composition text (ambiguous with **Composition**)

**Commit Text**:
The final Khmer string sent to the host application when the user confirms (Enter, Space, digit, etc.). May or may not equal the **Preedit** depending on whether the segmented or flat-refined path produces it.
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

**Bridge**:
The Rust subprocess (`khmerime-ibus-bridge`) that owns the **SharedTransliteratorData** and serves commands from the Python IBus adapter over stdin/stdout JSON. One bridge per `KhmerIMEEngine` instance.
_Avoid_: backend, service

**Learned History**:
The per-user `HashMap<String, usize>` counting how often each Khmer **unigram** has been committed. Boosts ranking in `suggest()` and `next_word_suggestions()` alongside the static corpus statistics. Persisted via `HistoryStore` (TSV today; see ADR-0002). Keys are individual Khmer words, never concatenated multi-word phrases — when a **Commit Text** spans multiple segments (from a **Segmented Session** or the **Commit Refiner**'s WFST output), each segment is learned separately.
_Avoid_: user dictionary, learned words (ambiguous with corpus)

**Phase A / Phase B startup**:
A two-stage warmup. Phase A builds a minimal **SharedTransliteratorData** fast enough to accept keystrokes (~100 ms). Phase B builds the full version in a background thread (~800 ms ngram, ~1300 ms SymSpell). Phase B is swapped in transparently when ready.

**Visible Refiner / Commit Refiner**:
Two distinct `Transliterator` views built from the same **SharedTransliteratorData** but with different decoder configurations. The visible refiner has a 75 ms latency budget for in-flight preview refinement; the commit refiner has no budget and is used to produce the final **Commit Text** on Enter.
_Avoid_: refiner (use the qualified form)

## Relationships

- A **Lexicon** is built into one **SharedTransliteratorData** at startup
- A **SharedTransliteratorData** contains exactly one **Search Index**
- A **SharedTransliteratorData** is shared by the live engine, **Visible Refiner**, and **Commit Refiner** (three views, one underlying data)
- A **Composition** may have zero or one **Segmented Session**
- A **Preedit** and **Commit Text** are derived from the same **Composition** but may diverge (see ADR-0001)
- A **Visible Segmented Commit** makes the visible **Segmented Session** authoritative over hidden commit refinement
- A **Visible Candidate Commit** makes the visible selected candidate authoritative over hidden commit refinement
- A **Hidden Commit Fallback** only applies when visible state is not useful Khmer **Commit Text**
- A **Bridge** owns exactly one **SharedTransliteratorData** for its lifetime
