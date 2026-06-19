# Ubiquitous Language — Android IME Adapter

Terms used consistently across Kotlin UI code, JNI/session bindings, tests, and
conversations about the Android keyboard adapter.

The Android adapter intentionally differs from the iOS keyboard in two important
places:

1. Candidates always live in the suggestion bar.
2. Suggest Character mode keeps the default QWERTY layout visible.

---

## Keyboard Area

The visible key region below the preedit bar and suggestion bar.

The keyboard area shows only the current key layout:

- QWERTY keys
- Numeric keys
- Symbol keys

The keyboard area does not show candidate lists on Android.

---

## Suggestion Bar

The horizontal candidate strip above the keyboard area.

All candidates appear here on Android:

- roman composition candidates
- focused-segment candidates
- Suggest Character mapped-character candidates
- Suggest Character Coeng Form candidates

Candidate selection is handled through `KhmerInputHandler`, not directly by the
Android view. The view renders suggestions; the handler owns behavior.

---

## Preedit Bar

The row above the suggestion bar showing the roman composition or hint text.

For normal roman input, it may show the active preedit. In Suggest Character mode
it stays empty because Suggest Character is direct character picking, not roman
composition.

---

## Keyboard Mode

The active Android key layout. Exactly one key mode is visible at a time.

| Mode | Description |
|---|---|
| **QWERTY** | Default roman-input key layout. Includes the Suggest Character key. |
| **Numeric** | Number/symbol key layout reached from `123`. |
| **Symbols** | Secondary symbol key layout reached from `#+=`. |
| **Suggest Character** | Direct Khmer character-suggestion mode using the same QWERTY key layout. |

Android may reserve a future **Panel** mode, but the current Android adapter does
not use a candidate panel as the normal suggestion surface.

---

## Suggest Character Key

The key currently shown in the QWERTY shift-key position.

This is the Android meaning of the old lightbulb/panel key. Its domain behavior
is not "toggle candidate panel"; it toggles direct character suggestion while the
default QWERTY layout remains visible.

Current Android behavior:

1. From QWERTY, tapping Suggest Character turns Suggest Character mode on.
2. If roman composition is active, turning Suggest Character mode on discards that composition.
3. Tapping Suggest Character again turns the mode off and returns to normal QWERTY behavior.
4. Candidates remain in the suggestion bar.
5. Only the Suggest Character key should visually toggle; the rest of the keyboard layout stays the same.

Implementation note: code may still contain older names such as `TogglePanel` or
`CharPick`. New Android code should prefer the domain term **Suggest Character**
unless touching existing API names would make the change larger
than the behavior being implemented.

---

## Suggest Character Mode

The Android mode for direct Khmer character suggestion.

Suggest Character mode keeps the normal QWERTY layout on screen. Tapping a
QWERTY letter sends that letter to the session as a character-suggestion query
and renders the resulting Khmer character candidates in the suggestion bar.
QWERTY letters do not insert roman text while Suggest Character mode is on.

Selecting a Suggest Character candidate inserts the chosen Khmer character into
the text field and resets suggestions so the user can pick another character.
Coeng Forms appear under the same roman letters as their base consonants so the
user can type consonant clusters quickly.

---

## Candidate

A Khmer option returned by the session for the current input context.

On Android, candidate placement is platform-specific:

- Candidates are always rendered in the suggestion bar.
- Candidate buttons are not rendered inside the keyboard area.
- Candidate taps route through `KhmerInputHandler.selectCandidate`.

---

## Panel

A possible future Android mode, not the current candidate presentation model.

Do not use "panel" as a synonym for suggestions on Android. If a future Android
panel is introduced, it should be a real mode with its own purpose and tests,
while candidates can still remain in the suggestion bar unless that future design
explicitly changes this document.

---

## Composition

The in-progress roman input that has not yet been committed.

Normal roman composition is committed by return/space behavior through the
handler. Turning on Suggest Character mode discards active roman composition
because Suggest Character is direct character input.

---

## Session Render State

The snapshot returned by the Rust session after input.

Android uses the render state to update:

- preedit bar text
- suggestion bar candidates
- commit text
- segment metadata for future focused-segment behavior

The render state does not decide whether candidates belong inside the keyboard
area. Android presentation policy owns that platform decision.
