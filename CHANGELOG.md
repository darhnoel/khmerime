# Changelog

All notable changes to KhmerIME are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow the
product version described in [docs/release_automation.md](docs/release_automation.md).

## [Unreleased]

### Added

- **Optional AI model support (Smart mode).** The engine accepts a separately supplied
  span-proposal model behind a pluggable seam ([ADR-0016](docs/adr/0016-runtime-model-provider-behind-a-lexicon-verified-marker.md)).
  The model runs on a debounced pause, off the keystroke path, and its suggestions carry
  a ✦ marker. On iOS, Android and macOS a red ✦ flags words outside the Lexicon. Wired on
  iOS, Android, macOS, Windows and Linux; the standard build stays dictionary-only.
- **Phrase Wheel** on iOS and Android: whole-phrase alternatives above the keyboard. Tapping
  a card previews it in the strip; Space or Return commits it
  ([ADR-0014](docs/adr/0014-phrase-wheel-replaces-default-candidate-row.md),
  [ADR-0015](docs/adr/0015-phrase-wheel-refined-after-live-testing.md)).
- **Two-level candidate surface** on macOS and Linux: whole-phrase rows first, then the
  words of one segment.
- **Mobile keyboard chrome:** literal keycaps, a quick-access tray for Khmer signs, haptics,
  and an always-visible next-keyboard globe whose long-press opens the keyboard picker
  ([ADR-0021](docs/adr/0021-mobile-literal-keycaps-and-quick-access-chrome.md),
  [ADR-0022](docs/adr/0022-next-keyboard-globe-always-visible.md)).
- **Mobile apps:** Standard/Smart toggle, swipeable Tips and Characters carousels with roman
  chips per character, and an iOS App Store release pipeline.
- **Web editor (beta):** a focused document workspace with spell review, saved words, color
  palettes, and the core of a flick keyboard.
- **Lexicon:** the RAC Khmer Dictionary staging pipeline and lexicon-editor bulk tools; the
  approved lexicon now holds about 75,000 rows.

### Changed

- **macOS:** ↑/↓ step one candidate and PageUp/PageDown move by page
  ([ADR-0020](docs/adr/0020-macos-arrows-step-pageupdown-pages.md)).
- **iOS and Android:** the bottom keyboard row is rebalanced so both platforms match.
- **Segment editing:** Left/Right stay inert until Tab enters Segment Edit Mode, and Space
  cycles a segment's words while editing.
- **Startup:** the dictionary loads with a far smaller memory peak. On iOS, loading the
  dictionary data peaks at about 21 MB instead of 54 MB and takes about 30 ms instead of
  340 ms, which keeps the keyboard extension clear of the system memory limit.
- **AI suggestions** arrive faster: one model inference per word instead of four.

### Fixed

- **iOS:** tapping a later word in the strip focuses it, and a second tap edits it.
  Previously only the first word could be edited by tapping.
- **macOS:** AI suggestions appear reliably. The model refine had a 250 ms budget, shorter
  than one inference, so most results were discarded.
- **AI model:** long Khmer words are reachable. Words beyond seven code points (for example
  ប្រជាពលរដ្ឋ, វិនិច្ឆ័យ) were cut short.
- **iOS:** the Xcode project opens in Xcode 27.
- **macOS:** Backspace clears the composition on its last character, ⌘-shortcuts pass
  through, and the model refine runs off the main thread.
- **Windows:** typing during warmup composes instead of leaking roman text into the
  document; phrase rows show their own roman spelling.
- **Linux:** the debounced refine runs off the IBus stdio pipe, so typing stays responsive.
- **Android:** typing lag, keyboard flicker, keyboard-open jump, stale suggestions after an
  external clear, and Backspace on selected text.
- **Lexicon data:** corrected roman keys and a cleaned English–Khmer seed lexicon.

## [1.0.0-rc.1] - 2026-07-04

- First release candidate.

[Unreleased]: https://github.com/darhnoel/khmerime/compare/v1.0.0-rc.1...HEAD
[1.0.0-rc.1]: https://github.com/darhnoel/khmerime/releases/tag/v1.0.0-rc.1
