# Windows (TSF)

## Adapter

- Planned crate: `adapters/windows-tsf`
- Runtime boundary: TSF COM callbacks/sinks -> `khmerime_session` commands
- Dioxus note: Dioxus remains separate app/runtime and is not embedded in adapter.

## Official References

- IME overview: <https://learn.microsoft.com/en-us/windows/apps/develop/input/input-method-editors>
- IME requirements: <https://learn.microsoft.com/en-us/windows/apps/develop/input/input-method-editor-requirements>
- `ITfTextInputProcessor`: <https://learn.microsoft.com/en-us/windows/win32/api/msctf/nn-msctf-itftextinputprocessor>

## Lifecycle Mapping (Planned)

| Native callback | Session intent |
| --- | --- |
| `ITfTextInputProcessor::Activate` | `focus_in + enable` |
| `ITfTextInputProcessor::Deactivate` | `focus_out + disable` |
| `ITfKeyEventSink::OnKeyDown` | `process_key_event` |
| context/cursor update | `set_cursor_location` |

## Milestones

1. Add TSF COM text-service shell.
2. Implement callback-to-session mapping.
3. Wire composition/preedit/candidate updates.
4. Add Notepad/browser smoke checks.

## First Contributor Tasks

1. Add TSF text-service skeleton implementing `ITfTextInputProcessor`.
2. Route key sink callbacks into `NativeKeyEvent`.
3. Wire snapshot preedit/candidates into TSF composition UI.
4. Add single-shot commit flow for `SessionResult.commit_text`.
5. Add smoke checklist for Notepad + Chromium textarea.
