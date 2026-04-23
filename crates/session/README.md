# khmerime_session

Platform-agnostic IME session state machine.

## Owns
- Native key event semantics
- Session command surface
- Session snapshot/result contract
- History persistence trait boundary (`HistoryStore`)

## Does Not Own
- Transliteration internals
- Linux IBus process wiring
