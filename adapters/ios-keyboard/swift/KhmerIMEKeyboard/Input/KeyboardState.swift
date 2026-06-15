// KeyboardState
// =============
// The five mutually exclusive visual/input states the keyboard can be in.
// Kept in its own file so KeyboardInputHandler (no UIKit) and
// KeyboardViewController (UIKit) can both import it.
//
// Transitions:
//   qwerty ⇄ numeric ⇄ symbols     (123 / #+= / ABC layer keys)
//   qwerty → panel                  (✦ when composition is active)
//   panel  → qwerty                 (✦ dismiss, or candidate committed)
//   qwerty → charPick               (✦ when no composition is active)
//   charPick → qwerty               (✦ dismiss, or ⌫ from alphabet)

enum KeyboardState {
    case qwerty
    case numeric
    case symbols
    case panel
    case charPick
}
