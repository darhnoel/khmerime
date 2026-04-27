# iOS (Keyboard Extension)

For the shared platform workflow, read [`docs/platforms/README.md`](README.md).

## Adapter

- Planned crate: `adapters/ios-keyboard`
- Runtime boundary: native keyboard extension callbacks -> `khmerime_session` commands
- Dioxus note: Dioxus remains separate app/runtime and is not embedded in adapter.

## Official References

- `UIInputViewController`: <https://developer.apple.com/documentation/uikit/uiinputviewcontroller>
- `UITextDocumentProxy`: <https://developer.apple.com/documentation/uikit/uitextdocumentproxy>
- Custom Keyboard guide:
  <https://developer.apple.com/library/archive/documentation/General/Conceptual/ExtensibilityPG/CustomKeyboard.html>

## Lifecycle Mapping (Planned)

| Native callback | Session intent |
| --- | --- |
| `viewDidAppear` | `focus_in` |
| `viewWillDisappear` | `focus_out` |
| key input callback | `process_key_event` |
| `selectionDidChange` | `set_cursor_location` |

## Milestones

1. Add Swift extension shell and callback bridge.
2. Implement callback-to-session mapping and render state wiring.
3. Implement candidate/preedit/commit behavior through `textDocumentProxy`.
4. Add manual smoke checklist in sample host apps.

## Package Criteria

Do not add `make ios-package` yet. Add iOS packaging only after a real keyboard
extension target exists and can run inside a host app or simulator.

The target package workflow should use Xcode archive/export and write artifacts
under `dist/ios/`.

The smoke checklist must prove the extension can be enabled, opened from a text
field, render candidates, and commit Khmer text through `UITextDocumentProxy`.

## First Contributor Tasks

1. Wire `viewDidAppear`/`viewWillDisappear` lifecycle to session focus commands.
2. Add key-event conversion into `NativeKeyEvent`.
3. Add candidate strip rendering from `SessionSnapshot.candidates`.
4. Add commit integration via `textDocumentProxy.insertText`.
5. Add smoke script/checklist for Safari + Notes text input.
