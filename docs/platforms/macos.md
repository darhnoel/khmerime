# macOS (InputMethodKit)

For the shared platform workflow, read [`docs/platforms/README.md`](README.md).

## Adapter

- Planned crate: `adapters/macos-imk`
- Runtime boundary: InputMethodKit callback layer -> `khmerime_session` commands
- Dioxus note: Dioxus remains separate app/runtime and is not embedded in adapter.

## Official References

- InputMethodKit: <https://developer.apple.com/documentation/inputmethodkit>
- `IMKInputController`: <https://developer.apple.com/documentation/inputmethodkit/imkinputcontroller>

## Lifecycle Mapping (Planned)

| Native callback | Session intent |
| --- | --- |
| `activateServer:` | `focus_in` |
| `deactivateServer:` | `focus_out` |
| `handleEvent:` | `process_key_event` |
| cursor/selection update | `set_cursor_location` |

## Milestones

1. Add macOS IMK host/bundle shell.
2. Implement callback routing to session commands.
3. Implement marked/preedit text and commit flow.
4. Add manual editor smoke matrix.

## Package Criteria

Do not add `make macos-package` yet. Add macOS packaging only after a real
InputMethodKit bundle exists and can be enabled as a macOS input source.

The target artifact should be a signed `.pkg` under `dist/macos/`.

The smoke checklist must prove the input method can be installed, selected from
macOS input sources, show marked/preedit text, and commit Khmer text in TextEdit
and a browser text field.

## First Contributor Tasks

1. Bind `activateServer:` and `deactivateServer:` lifecycle to session state.
2. Convert `handleEvent:` key payloads to `NativeKeyEvent`.
3. Add marked/preedit rendering from `SessionSnapshot.preedit`.
4. Add candidate UI refresh from snapshot candidates.
5. Add smoke checklist for TextEdit + browser fields.
