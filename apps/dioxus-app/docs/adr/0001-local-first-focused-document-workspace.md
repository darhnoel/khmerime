# The Online Beta becomes a local-first focused document workspace

> Webapp context ADR 0001 (`apps/dioxus-app/docs/adr/`). References to bare
> `ADR-00NN` below point to the **root system sequence** (`docs/adr/`); see
> `CONTEXT-MAP.md`. (Originally numbered 0023 in the root sequence before the
> webapp got its own context.)

The **Online Beta** began as a quick trial surface, but a full-bleed canvas left its editor, settings, and candidate surfaces visually unrelated on wide screens. We will evolve it into a focused, local-first document workspace: a minimal top bar, a collapsible 260 px navigation sidebar, and a centered plain-text **Document** surface capped near 920 px. This deliberately supersedes root ADR-0010's shared Silk Veil visual identity for the Online Beta; Silk Veil remains the **Download Landing Page** identity, while ADR-0010's solid candidate/preedit surfaces and WCAG contrast requirement remain in force.

## Decision

- The desktop shell uses a restrained page surface centered in the space beside the sidebar. On narrow screens the sidebar becomes an overlay drawer and the Document becomes a seamless full-width surface. The sidebar fully disappears when collapsed.
- The top bar contains only the sidebar toggle, KhmerIME identity, document-level actions when they exist, exceptional engine/save status, and Settings. Input modes, Rules, and Saved Words live in the sidebar. Settings is a modal on desktop and a bottom sheet on mobile.
- **Rules & Shortcuts** opens as a non-modal help sheet over the right edge of the workspace; it never changes the Document width or caret position. The sheet is approximately 400 px wide on desktop and full-width below 700 px, closes from its × action, Escape, or the active sidebar item, and groups help by Normal typing, Phrase/Segment Edit, Manual typing, and Romanization rules. Its internal scrollbar is visually hidden, with a bottom-edge fade and scroll cue preserving the affordance that more content follows.
- The editor remains a plain `<textarea>` to protect Khmer **Composition** behavior. The page, rather than a small internal editor viewport, owns scrolling; hidden native scrollbars are paired with a subtle overflow fade and temporary cue.
- Transliteration uses a caret-adjacent, IMK-shaped **Candidate Surface**. Next-word predictions use a separate footer dock attached to the Document. One ranked list must never render on both surfaces, and NextWord mode remains pointer-only: Enter inserts a newline and Tab/Space do not select predictions.
- The Candidate Surface adopts the macOS adapter's two-level interaction. A detected **Segmented Session** opens in Phrase mode and lists only whole **Phrase Candidates**; it must not jump directly to alternatives for the first segment. `Space`/Up/Down cycle phrases, digits choose a visible phrase, and Enter commits the selected phrase. `Tab` deliberately enters Segment Edit, where Left/Right move the focused segment and Space/Up/Down cycle that segment's words; pressing `Tab` again returns to Phrase mode.
- The raw Roman **Composition** remains visible in the Document until commit. Candidate selection and Segment Edit must never paint a replacement or segmented preview over the textarea. Segment context exists only inside the Candidate Surface after Segment Edit is explicitly entered.
- The Candidate Surface is a stable five-row page aligned to the active caret. It uses the app theme's opaque elevated surface, a subtle boundary and shadow, rounded corners, a filled selected row, visible `1`–`5` keys, and a quiet page count when more candidates exist. It does not animate or change anchor while switching levels, flips above the line when needed, clamps inside the viewport, and keeps a fixed minimum height during a composition.
- Web candidates remain clickable in addition to the IMK keyboard controls. Clicking a phrase commits it; a pencil affordance on the selected phrase enters Segment Edit; clicking a segment candidate updates that segment without committing the phrase; and the Segment Edit header provides a quiet back control. Narrow screens retain the same caret-anchored surface rather than replacing it with a docked candidate strip.
- Manual Character Typing does not create a detached progress strip above the Document. Once manual state diverges from the raw Roman composition—because a character was built or Roman input was consumed—the Candidate Surface gains a compact context header containing Built Khmer, Remaining Roman, and the expected Next character kind. Before that point the header stays hidden because it would duplicate the textarea. Flat and Manual candidate surfaces size to their actual rows; only Phrase/Segment switching reserves the stable five-row footprint.
- A persistent footer shows hints appropriate to the active mode. It must never advertise a shortcut that the current mode disables.
- The interface offers Light, Dark, and System themes, with System as the default. Both palettes use the same semantic tokens; core text and selectable candidates remain on opaque surfaces meeting WCAG AA contrast.
- Khmer is the default interface language. Sidebar actions use icons plus labels; compact universal top-bar actions may use icons with Khmer tooltips and accessible labels.

## Document direction

The first UI slice does not expose nonfunctional document controls. Later functional slices introduce manually titled Documents, local autosave, in-session Undo/Redo, Recent Documents, one Collection per Document (with Unfiled), many-to-many Tags, search, Trash, export/import, and periodic Document Versions. Browser storage is local-first behind a storage boundary; accounts, a remote database, and cross-device synchronization remain later work. Existing saved editor text must migrate without loss when the Document model arrives.

## Consequences

- The **Online Beta** is no longer visually coupled to the marketing page, so theme changes do not need to move in lockstep.
- Document organization UI stays hidden until its persistence behavior exists; the initial sidebar contains only working input modes and tools.
- Responsive layout and candidate-mode predicates are behavior, not polish, and require browser-level regression coverage.
