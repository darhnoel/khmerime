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
The visible Khmer choices for the active **Composition** or focused **Segmented Session** segment. On mobile, its choices form a centered group when they fit and a left-aligned scroll row when they overflow; in romanization mode it remains visible even with one obvious choice because it tells the user what Enter will commit while the inline **Preedit** stays roman.
_Avoid_: suggestions (too broad), inline candidates

**Phrase Candidate**:
One complete Khmer rendering of the *entire* **Composition** — a whole-input hypothesis carrying one or more internal segments. Complete single-word readings and multi-word segmented readings coexist as Phrase Candidates; finding a segmentation never discards the whole-word alternatives. Distinct from a **Candidate**, which is one Khmer choice for a *single* focused segment.
_Avoid_: sentence candidate, sequence, full-composition candidate, phrase suggestion, n-best (implementation term)

**Candidate Surface**:
The two-level candidate presentation for a **Segmented Session**: complete **Phrase Candidate**s are the default level, and the focused segment's word **Candidate List** is a second level reached on demand (Tab on desktop; a double-touch into **Phrase Edit** on mobile). The phrase level merges complete interpretations without forcing single-segment or multi-segment readings to outrank the other; a flat **Composition** has no second level. This is *adapter presentation policy*, not new ranking — the shared engine still owns Phrase Candidates, **Segment Edit Mode**, selection, and commits.
_Avoid_: candidate popup (too generic), suggestion box, phrase panel

**Phrase Wheel**:
The default mobile presentation of the **Candidate Surface**'s phrase level (iOS + Android): a horizontal row of the *alternative* **Phrase Candidate**s — the whole-phrase Khmer hypotheses **other than** the currently selected one, which the **Strip**'s Khmer Row already shows. The cards are centered when they all fit the width and left-padded + horizontally scrollable when they overflow. Tapping a card selects that phrase as the **Strip** preview; Space/Enter commit the selected reading. Shown only when at least one alternative exists; otherwise the **Strip** stands alone and the wheel is hidden. It demotes the word-level candidate row to **Phrase Edit** only (see ADR-0014). Distinct from a **Candidate List**, which browses word choices for one segment.
_Avoid_: candidate carousel, suggestion strip, alarm-clock picker

**Phrase Edit**:
The mobile form of **Segment Edit Mode**, reached by a double-touch on the centered Khmer in the **Phrase Wheel**. The phrase expands into separated, tappable words with a word-level **Candidate List** for the focused segment; tapping a word moves focus, typing re-spells the focused word, and double-touch returns to the wheel. It is per-phrase and never sticky — any commit resets the **Composition** and the next one starts back at the **Phrase Wheel**.
_Avoid_: expanded mode, level 2 (internal label), word edit mode

**Key Preview Popup**:
A transient visual above a pressed character-producing on-screen key that mirrors the key label so the user can confirm which key is being touched. In **CharPick Mode** it still mirrors the roman key label, not a Khmer candidate; it is visual feedback only and does not commit text, change input mode, select candidates, or interact with the **Phrase Wheel** or **Candidate Row**.
_Avoid_: candidate popup, suggestion popup, key tap animation

**Punctuation Suggestions**:
A shared-engine and desktop behavior that produces `។`, `៕`, `.`, `?`, `!`, and `…` when a period enters transliteration. Mobile keyboards do not invoke it from their visible `.` key: mobile punctuation keys are **Literal Keycap**s, while Khmer punctuation is directly available from the **Quick Access Tray**.
_Avoid_: period popup, punctuation key preview

**QWERTY Character Grid**:
The visual grid used by the mobile QWERTY layer where roman letter keys keep one consistent character-key width across rows, while edge controls may be wider for reachability. It is a keyboard-layout concept, not an input mode or candidate surface.
_Avoid_: packed row, stretched letters, English layout clone

**Coeng Form**:
A Khmer subscript consonant used to type consonant clusters. A Coeng Form is the invisible coeng sign plus a base consonant, rendered together as a subscript shape (for example `្ក`). In **CharPick Mode**, Coeng Forms appear under the same roman letters as their base consonants so users can build clusters quickly.
_Avoid_: bare coeng sign when you mean the full subscript consonant

**Roman Hint**:
The exact romanized key or keys displayed beside a Khmer candidate to show why that candidate is available. Roman Hints are display metadata for the **Candidate List**; they do not replace the raw roman **Preedit** and do not change the **Commit Text**. If no exact Roman Hint exists for a candidate, the UI must not invent one; it should show a derived marker instead.
A Roman Hint describes the *candidate*, never the input: two different Khmer readings of one
roman input do not share a hint. A **Phrase Candidate** spanning several segments carries one
canonical spelling per segment in reading order; a single-segment phrase carries the same hints
the same word would carry in a flat **Candidate List**, so a word reads identically whether or not
the composition happened to segment. When any segment has no exact hint the whole row has none —
a partial hint would be an invented one.
_Avoid_: invented hint, transliteration label, candidate subtitle

**Pronunciation Variant**:
One alternative spoken form of a Khmer headword. Dictionary pronunciations joined by `ឬ` are separate Pronunciation Variants even when they share one written word; dictionary senses that share both the written word and pronunciation are not separate variants. Each Pronunciation Variant may produce its own candidate **Roman Alias** for Lexicon review.
_Avoid_: duplicate word, pronunciation sense

**Roman Alias**:
A reviewed roman spelling that maps to one Khmer **Lexicon** target. One target may have multiple Roman Aliases, including aliases derived from distinct **Pronunciation Variant**s.
_Avoid_: pronunciation, Roman Hint

**Commit Text**:
The confirmed text sent to the host application when the user confirms. In romanization mode, Enter is the normal confirmation key; Space and digit keys select candidates or segments during an active **Composition**. Commit text is normally Khmer, selected from candidates, a **Segmented Session**, or the **Commit Refiner**; it may fall back to roman text when no Khmer output is available.
_Avoid_: output text, submission

**Visible Segmented Commit**:
The rule that when a **Segmented Session** is visible, Enter commits the currently selected segment outputs exactly as shown. The visible segmented phrase is authoritative; Enter must not replace it with a hidden refinement.
_Avoid_: WYSIWYG commit, segment preview commit

**Visible Candidate Commit**:
The rule that when the candidate list has a selected visible candidate, committing takes that candidate exactly. Enter commits it; for a single-word **Composition** (no **Segmented Session**), tapping the shown word in the **Preedit** commits it directly (see ADR-0012). Hidden refinement may fill in when no visible candidate is available, but must not replace the candidate the user can see.
_Avoid_: hidden override, invisible correction

**Hidden Commit Fallback**:
The limited use of the **Commit Refiner** when visible state is not meaningfully commit-ready. It may recover a Khmer phrase when the visible candidate is empty, raw roman, or otherwise not useful as Khmer **Commit Text**, but it must not override a visible Khmer candidate or **Segmented Session**.
_Avoid_: final refinement, commit correction

**Commit Rules**:
The ordered precedence applied at commit time to pick the **Commit Text** from a **Composition**: **Visible Segmented Commit** first, then **Visible Candidate Commit**, then **Hidden Commit Fallback**, with the raw roman string as the final floor. Each step is one of the rules above; the earliest one that yields useful Khmer wins, so visible state always outranks hidden refinement.
_Avoid_: commit resolution, commit policy, commit precedence

**Editor Action**:
The action a host text field asks the keyboard's Enter/Return key to perform — Android's `imeOptions` action (Search, Go, Send, Done, Next, …) or the iOS return-key type. When a field declares an Editor Action and is not multiline (and does not suppress it), Enter performs that action instead of inserting a newline; multiline fields, fields that suppress the action, and fields with no declared action take a literal newline. During an active **Composition**, one Enter first applies the **Commit Rules** and then performs the Editor Action if present — so a single Enter both commits the Khmer **Commit Text** and runs the field's action (e.g. a search). The keyboard never inserts a newline in place of a declared action; doing so is the cause of the Android "Enter outputs a space in Google Search" bug (a committed newline collapses to a space in a single-line field).
_Avoid_: enter key / return key (those are the physical key, not the requested action), submit, search action (too narrow)

**Composition-Consuming Enter**:
The desktop counterpart to **Editor Action**: with a **Composition** active, Enter applies the **Commit Rules**, sends the **Commit Text**, and is fully consumed, so the host application never receives a Return. With no composition active, Enter is not consumed and passes through for the application to interpret (send, newline, default button). Desktop platforms have no way to ask a focused field what its Return means — unlike Android's `imeOptions` or the iOS return-key type — so the input method stays out of the way instead of performing the action itself. This is why committing Khmer and sending a message take two separate Enters on the desktop (see ADR-0017). Applies to macOS and any other desktop adapter; **Editor Action** remains mobile-only.
_Avoid_: swallowed enter, enter passthrough (names one half of the rule), desktop editor action (there is no editor action to read)

**Bridge**:
The Rust subprocess (`khmerime-ibus-bridge`) that owns the **SharedTransliteratorData** and serves commands from the Python IBus adapter over stdin/stdout JSON. One bridge per `KhmerIMEEngine` instance.
_Avoid_: backend, service

**Learned History**:
The per-user `HashMap<String, usize>` counting how often each Khmer **unigram** has been committed. Boosts ranking in `suggest()` and `next_word_suggestions()` alongside the static corpus statistics. Persisted via `HistoryStore` (TSV today; see ADR-0002). Keys are individual Khmer words, never concatenated multi-word phrases — when a **Commit Text** spans multiple segments (from a **Segmented Session** or the **Commit Refiner**'s WFST output), each segment is learned separately. Distinct from a **Lexicon Pack**: history is implicitly-counted usage that re-ranks existing candidates; a pack is an explicit set of roman→Khmer entries the user opts into.
_Avoid_: user dictionary, learned words (ambiguous with corpus)

**English Mode**:
An input mode in which all keystrokes (letters, symbols, numbers, space, backspace, return) are routed directly to the host text field without Khmer processing or roman-buffer accumulation. Toggled by the EN key, which occupies the globe-key slot when the system keyboard switcher is not needed. English Mode is orthogonal to the visual layer — switching between QWERTY, 123, and #+= does not exit English Mode — and is the only mobile mode in which the **Khmer Input Chrome** is absent. Pressing ✦ while in English Mode exits English Mode and enters CharPick (since no Composition is active). Pressing EN while composing abandons the active Composition silently: the Rust session resets, the roman Preedit remains in the host text field as literal text, and English Mode begins.
_Avoid_: latin mode, passthrough mode, direct-input mode

**Lexicon Pack**:
A named, versioned overlay of exact-match roman→Khmer entries the engine consults alongside the base **Lexicon**. Two kinds, one mechanism: the always-on, editable **personal pack** (the user's own added words) and read-only **curated packs** (tech, medical, loanword sets) the user toggles on. Packs match only on an exact roman key (no fuzzy **Search Index** participation); pack candidates rank above base **Lexicon** candidates, personal pack first, then enabled curated packs in user-defined order, with **Learned History** still applied as a cross-cutting boost. Each pack carries a stable ID and version so a future remote registry can deliver and update packs without a format change. Stored as one `roman\tKhmer` TSV file per pack in the **Config Store**.
_Avoid_: user dictionary, secondary lexicon, word list, code-switching

**Config Store**:
The shared, cross-platform per-user configuration read by the engine at runtime: a `config.toml` (next-word suggestion on/off, count, learn-from-typing flag, and the ordered list of enabled **Lexicon Pack** IDs) plus the per-pack TSV files. Owned by a dedicated `khmerime_config` crate so persistence is not tied to any one platform adapter. Reachable by Desktop/Linux/Windows via the XDG config dir (`~/.config/khmerime/`, the existing **Learned History** location) and by the macOS IME and iOS keyboard via a shared **App Group** container. The web app does not read the Config Store — browser storage is origin-sandboxed and stays islanded.
_Avoid_: settings file, preferences, user dictionary

**Standard / Smart Mode**:
A user-selectable setting (not an `InputMode`) choosing whether the runtime model provider contributes candidates. **Standard** (the default) is the pure lookup + fuzzy engine over human-reviewed **Lexicon** data. **Smart** additionally enables the injected span-proposal provider (ADR-0016): the primary keystroke path stays Standard, and the model runs only as a debounced **Visible Refiner** off the hot path, its out-of-**Lexicon** output shown with a red ✦ (`lexicon_verified == false`). Smart is **inert without a registered provider** — in the OSS build the toggle has no visible effect and the engine stays Standard, so the setting is provider-agnostic and names no model. Persisted per-platform: Android `SharedPreferences`, iOS a shared **App Group** `UserDefaults` suite (so the host-app Settings toggle reaches the keyboard extension). The keyboard applies the saved choice via `set_model_mode` when its session is (re)created. On **macOS** there is no settings surface (the IMK app is an `LSUIElement` accessory), so Smart is **implicit**: it is on whenever a provider is armed and off otherwise — the AI build is Smart, the OSS build is Standard, with no per-user toggle. The user-facing label is Khmer បញ្ញាសិប្បនិម្មិត (AI); "Smart" is the internal English term for the mode.
_Avoid_: model mode (internal API term only), neural mode

**CharPick Mode**:
An input mode (`InputMode::CharPick`) for typing Khmer text that is not in the **Lexicon** — names, place names, loanwords. The user types one roman letter; the session looks up all Khmer characters and **Coeng Forms** whose phonetic relation includes that letter and returns them as the **Candidate List**. Tapping a candidate commits that single Khmer character or Coeng Form immediately to the host application with no **Composition** or preedit accumulation. Each keystroke is an independent lookup; there is no progressive multi-letter narrowing. On iOS and Android, the ✦ key toggles CharPick Mode: pressing ✦ enters it (abandoning any active **Composition** and clearing the roman buffer), pressing ✦ again exits it. While in CharPick Mode the keyboard layer remains unchanged (qwerty stays visible) and the ✦ key is visually highlighted. Letter keypresses are routed to the session without inserting text into the host field — only the **Candidate List** updates. Backspace while candidates are visible clears them and resets for a new lookup; backspace with an empty **Candidate List** deletes one character from the host text field.
_Avoid_: name mode, character picker, direct character input

**Phase A / Phase B startup**:
A two-stage warmup. Phase A builds a minimal **SharedTransliteratorData** fast enough to accept keystrokes (~100 ms). Phase B builds the full version in a background thread (~800 ms ngram, ~1300 ms SymSpell). Phase B is swapped in transparently when ready. The staging exists to satisfy **Warmup Keystroke Capture** on hosts where the adapter is not already resident.

**Warmup Keystroke Capture**:
The rule that a keystroke arriving before the engine is ready is still the IME's to handle. It must become part of a **Composition**, never reach the host application as raw roman. An adapter may satisfy this by composing on a Phase A engine or by briefly waiting for the full engine, but declining the key is a defect: text the user believed was being composed is already committed and can no longer be converted.
_Avoid_: warmup passthrough, cold-start fallback

**Visible Refiner / Commit Refiner**:
Two distinct `Transliterator` views built from the same **SharedTransliteratorData** but with different decoder configurations. The visible refiner has a 75 ms latency budget for in-flight preview refinement; the commit refiner has a larger budget and produces the final **Commit Text** on Enter. Both budgets are wall-clock deadlines; when the commit refiner's trips, the commit degrades to the visible result rather than waiting (see ADR-0005).
_Avoid_: refiner (use the qualified form)

**Word Rescuer**:
The scope of the runtime span-proposal provider as currently shipped: it transliterates one whole roman span into a single real **Lexicon** Khmer word — it does **not** segment. Word boundaries are still found by the deterministic **Weighted Span Decoder**; the provider only rescues a word the decoder ranked poorly or missed, contributing at most one whole-**Composition**-span candidate per debounced refinement. So a model-assisted result surfaces as one marked candidate in the **Candidate List**, never as a re-segmented phrase. The `from_model` marker therefore rides a candidate, not a segmentation.

**Download Landing Page**:
The public KhmerIME page that helps users choose a platform download, try the **Online Beta**, and follow install steps. It is download-first, with the visitor's platform download as the primary action and the **Online Beta** as a secondary trial path.
_Avoid_: homepage, product site

**Online Beta**:
The focused, in-browser Khmer writing workspace (the dioxus-app, deployed at the beta URL) where a user writes with KhmerIME without installing a platform IME. It is local-first and grows from one persistent writing surface into named **Document**s; it remains browser-sandboxed and does not replace installed platform IMEs.
_Avoid_: playground, demo, web app

**Document**:
A manually titled body of plain Khmer or roman text in the **Online Beta**. A Document belongs to at most one **Collection**, may carry many **Tag**s, and has recoverable **Document Version**s.
_Avoid_: note, file, editor text

**Collection**:
The single organizational home of a **Document** in the **Online Beta**. A Document without an explicitly chosen Collection belongs to Unfiled.
_Avoid_: folder (implies filesystem semantics), group

**Tag**:
A reusable label that may classify many **Document**s, while each Document may carry many Tags. Tags complement Collections rather than replacing their single-home hierarchy.
_Avoid_: category, label

**Document Version**:
A recoverable, persisted checkpoint of a **Document** across editing sessions. It is distinct from Undo/Redo, which only reverses edits in the current session.
_Avoid_: undo history, autosave

**Silk Veil**:
The glassmorphic visual identity for KhmerIME's marketing surface, the **Download Landing Page**. A deep-ink / charcoal-plum base, soft translucent pearl-glass panes with white rim highlights, an ember-amber primary action, sparse peacock-teal accents, and warm ivory text; the focused **Online Beta** document workspace deliberately uses its own restrained light/dark interface.
_Avoid_: Liquid Glass Theme, light download theme, glass accents

**Companion App**:
The host application UI a user sees when they launch the KhmerIME icon — distinct from the keyboard itself (the IME service, input handling, and on-screen key views). It has two parts: the **Intro Flow** and the **Dashboard**, grouped as top-level `intro` and `dashboard` packages/folders (`com.khmerime.{intro,dashboard}` on Android, `KhmerIME/{Intro,Dashboard}/` on iOS) — kept separate from the keyboard code by the module/target itself (the Android `app` module, the iOS `KhmerIME` target vs `KhmerIMEKeyboard`).
_Avoid_: settings app, host app (ambiguous with the OS host application a keyboard runs inside), welcome screen (too narrow)

**Product Version**:
The public KhmerIME release version shared by the engine, official platform adapters, release notes, and user-facing support language. It identifies the release family, not a particular packaged upload.
_Avoid_: adapter version, crate version, development version

**Build Number**:
The monotonically increasing identifier for a concrete packaged KhmerIME artifact. It distinguishes repeated package uploads or review candidates within the same **Product Version** and is not the user-facing release name.
_Avoid_: release version, product version, semantic version

**Intro Flow**:
The one-time onboarding shown on first launch of the **Companion App**: a Welcome (brand landing) screen followed by a Setup Guide that walks the user through enabling the keyboard, then hands off to the **Dashboard** (see ADR-0011). Shown once and remembered; not a recurring surface.
_Avoid_: onboarding (generic), welcome flow, tutorial

**Dashboard**:
The persistent hub of the **Companion App**, reached after the **Intro Flow** (or directly once onboarding is remembered). A tabbed surface — Settings, Tips, and Support today — that grows as companion-app features are added. Distinct from the **Intro Flow**, which is one-time.
_Avoid_: home screen, main menu, settings (the Dashboard is more than settings)

**Single Keycap**:
A typed character that does not compose into a **Composition** — a digit, punctuation mark, or symbol (any ASCII graphic that is not a letter). It stands alone and commits immediately, either as a mapped Khmer character (e.g. `1` → `១`) or as a **Literal Keycap**, with no **Preedit** accumulation or **Candidate List**, provided no **Composition** is already in progress. A letter typed after a Single Keycap starts a fresh **Composition**. Shared-engine Single Keycaps typed mid-Composition remain part of the roman buffer; mobile **Literal Keycap**s are the explicit exception defined below.
_Avoid_: symbol key, special key (those name the keyboard key; this names the input behavior)

**Literal Keycap**:
A mobile on-screen **Single Keycap** whose visible label is its exact **Commit Text**. Every mobile digit, punctuation, and symbol key is a Literal Keycap; none silently substitutes a Khmer digit or legacy Khmer mapping, because those characters belong in the **Quick Access Tray** instead. During a **Composition**, a Literal Keycap first commits the visible Khmer composition and then inserts its own label, rather than joining the roman buffer. This is a mobile adapter contract; shared-engine mappings remain available to desktop and physical-keyboard adapters.
_Avoid_: passthrough key, raw symbol

**Khmer Input Chrome**:
The stable rows above the mobile key grid: romanization mode has two for the **Strip** and candidate surface (or the **Quick Access Tray** while idle), **CharPick Mode** has one for its **Candidate List**, and **English Mode** has none. Switching among the QWERTY, `123`, and `#+=` visual layers does not change the row count. Row count changes only on an explicit input-mode transition, never because composition content appears or disappears.
_Avoid_: suggestion rows (too narrow), blank rows, header

**Quick Access Tray**:
The idle state of the mobile **Khmer Input Chrome**, offering directly insertable Khmer digits and marks that would otherwise be difficult or surprising to reach from visibly literal symbol keys. A tray tap inserts that exact Unicode character at the selection with no automatic spacing, **Composition**, or candidates, and leaves the tray available for repeated taps. Romanization mode shows its digit and mark rows, then yields both to the **Strip** and candidate surface when a **Composition** begins. Idle **CharPick Mode** reuses only the mark row, which a Roman key replaces with the CharPick **Candidate List**.
_Avoid_: default suggestions, filler row, symbol keyboard

**Yukaleapintu (យុគលពិន្ទុ)**:
The Khmer mark `ៈ`, used for example at the end of `វចនៈ`. It is directly available from the **Quick Access Tray** and is distinct from both the literal Latin colon `:` and the Khmer mark `៖`.
_Avoid_: សញ្ញាធ្មេញកណ្ដុរ, colon

**Optimistic Insert**:
On a composing platform (Android), showing the raw roman key in the host field the instant it is pressed, before the transliteration decode returns — so typing never lags behind the finger. When the decode lands, the raw roman is replaced by the committed Khmer. Correct for composing letters; deliberately skipped for a standalone mapped **Single Keycap**, whose committed glyph differs from the raw key and would otherwise flash the Latin form before swapping to Khmer.
_Avoid_: speculative insert, preview text (this is real inserted text, later corrected)

**Ignore List**:
The set of words the user has dismissed during the current **Spell Review** so they stop being flagged. Session-only — held in memory and cleared on reload, never persisted. Ignoring a word suppresses **every** flagged instance of that exact word in the document and keeps it un-flagged for the rest of the session; the user views the collected words on the ពាក្យផ្ទាល់ខ្លួន ("personal words") page and can un-ignore any of them. The user-facing label is Khmer ពាក្យផ្ទាល់ខ្លួន; "Ignore List" is the internal English term. Distinct from the **Lexicon Pack**'s personal pack (persistent, added to improve *input* candidates) and from **Learned History** (implicit usage counts) — the Ignore List is an ephemeral spell-review-only suppression and changes no ranking or candidate.
_Avoid_: personal dictionary, user dictionary, ignore words (persistent connotation), whitelist

## Relationships

- A **Lexicon** is built into one **SharedTransliteratorData** at startup
- A **Dictionary Image** can replace heap-owned parts of **SharedTransliteratorData** while preserving the same IME behavior
- A **SharedTransliteratorData** contains exactly one **Search Index**
- A **SharedTransliteratorData** is shared by the live engine, **Visible Refiner**, and **Commit Refiner** (three views, one underlying data)
- A **Composition** may have zero or one **Segmented Session**
- A **Composition** exposes a ranked list of **Phrase Candidate**s; the top-ranked one is the default preview, and the currently selected Phrase Candidate defines the current **Segmented Session**
- Selecting a different **Phrase Candidate** swaps the active **Segmented Session**; the selection never persists past commit — committing resets the **Composition** and the next one starts from the top-ranked **Phrase Candidate** again
- A **Segmented Session** may be in **Segment Edit Mode** for at most one of its segments at a time
- A **Preedit** and **Commit Text** are derived from the same **Composition** but may diverge (see ADR-0001)
- A **Visible Segmented Commit** makes the visible **Segmented Session** authoritative over hidden commit refinement
- A **Visible Candidate Commit** makes the visible selected candidate authoritative over hidden commit refinement
- A **Hidden Commit Fallback** only applies when visible state is not useful Khmer **Commit Text**
- A **Bridge** owns exactly one **SharedTransliteratorData** for its lifetime
- **Warmup Keystroke Capture** binds every adapter, but the cost of satisfying it is not shared: a **Bridge** or a resident macOS process pays warmup once per login, while a TSF adapter pays it again in every host application process it is loaded into
- The **Download Landing Page** and the **Online Beta** share the **Silk Veil** visual identity
- The **Download Landing Page** links to the **Online Beta** as its secondary trial path
- A **Config Store** holds zero or more **Lexicon Pack**s plus the next-word suggestion settings
- A **Lexicon Pack** overlays the base **Lexicon** at lookup time; it does not modify **SharedTransliteratorData**
- The engine applies the **Config Store** so every adapter (IBus, TSF, macOS IMK, iOS) inherits identical pack and suggestion behavior
- **Learned History** and the **Config Store** are sibling per-user state; both live in `~/.config/khmerime/` on desktop and a shared **App Group** container on Apple platforms
