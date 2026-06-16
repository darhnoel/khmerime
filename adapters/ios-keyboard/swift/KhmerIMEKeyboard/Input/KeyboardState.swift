// KeyboardState
// =============
// The four mutually exclusive visual/input states the keyboard can be in.
// Kept in its own file so KeyboardInputHandler (no UIKit) and
// KeyboardViewController (UIKit) can both import it.
//
// Transitions:
//   qwerty ⇄ numeric ⇄ symbols     (123 / #+= / ABC layer keys)
//   qwerty → charPick               (✦ key or strip long-press)
//   charPick → qwerty               (✦ dismiss, or ⌫ from alphabet)

enum KeyboardState {
    case qwerty
    case numeric
    case symbols
    case charPick
}
