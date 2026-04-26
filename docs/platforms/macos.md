# macOS (InputMethodKit)

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

## First Contributor Tasks

1. Bind `activateServer:` and `deactivateServer:` lifecycle to session state.
2. Convert `handleEvent:` key payloads to `NativeKeyEvent`.
3. Add marked/preedit rendering from `SessionSnapshot.preedit`.
4. Add candidate UI refresh from snapshot candidates.
5. Add smoke checklist for TextEdit + browser fields.
