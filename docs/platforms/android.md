# Android (InputMethodService)

## Adapter

- Planned crate: `adapters/android-ime`
- Runtime boundary: Android IME service callbacks -> `khmerime_session` commands
- Dioxus note: Dioxus remains separate app/runtime and is not embedded in adapter.

## Official References

- Create an input method:
  <https://developer.android.com/develop/ui/views/touch-and-input/creating-input-method>
- `InputMethodService` API:
  <https://developer.android.com/reference/android/inputmethodservice/InputMethodService>

## Lifecycle Mapping (Planned)

| Native callback | Session intent |
| --- | --- |
| `onStartInput` | `focus_in + enable` |
| `onFinishInput` | `focus_out` |
| key input callback | `process_key_event` |
| `onUpdateSelection` | `set_cursor_location` |

## Milestones

1. Add Android IME service shell (Kotlin/Java) in future phase.
2. Implement callback-to-session mapping bridge.
3. Wire candidate/preedit/commit behavior with `InputConnection`.
4. Add emulator/device smoke matrix.

## First Contributor Tasks

1. Create `InputMethodService` shell and lifecycle logging.
2. Bridge key events into `NativeKeyEvent` conversion.
3. Render candidates/preedit from `SessionSnapshot`.
4. Commit selected text with `InputConnection.commitText`.
5. Add smoke checklist for emulator + physical keyboard input.
