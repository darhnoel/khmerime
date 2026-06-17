# Fixing iOS keyboard key-press latency

## Symptom

On a physical device (iPhone X), fast typing felt broken in two distinct ways:

1. **Dropped taps** — double-tapping `t` to get `tt` produced only one `t`; a rapid
   run like `tttttttt` lost characters. The *2nd, 4th, 6th…* taps — every other one
   on the off-beat — were the ones that vanished.
2. **Sluggish feel** — once the drops were fixed, every character registered, but the
   keys still felt like they trailed the finger during fast typing.

Both reproduced only on device under rapid input, not in the simulator or unit tests,
which is why the root causes were behavioural (UIKit touch routing + animation timing)
rather than logic bugs.

## What it was *not*

Several plausible suspects were ruled out by experiment before the real causes were
found:

- **`proxy.insertText` XPC overhead.** A "preedit model" branch was tried that removed
  *all* per-keystroke writes to the host text field (roman shown only in the strip,
  Khmer committed once on space/return). Typing was *still* sluggish, proving the host
  text-field IPC was not the bottleneck. That branch also broke live roman display and
  hold-to-delete, so it was fully reverted.
- **Debug build / MallocStackLogging.** The device was already running a **Release**
  build with Full Access, so the known Debug-only slowdown did not apply.
- **Rust session compute.** The lag did not scale with composition length (it was a
  fixed per-tap cost and a fixed *every-other-tap* pattern), which pointed away from the
  per-keystroke segmentation/ranking work.

## Root cause 1 — the hit-test "dead ring" (dropped taps)

Each key is a `GlassKeyButton` whose press feedback scales the whole view down to 92%
via a `CGAffineTransform`. The release animation held that scaled state for **220 ms**.

UIKit hit-tests a transformed view through its **scaled** geometry: it maps the incoming
touch point back through the inverse transform and checks `point(inside:)` against the
bounds. When a key is shrunk to 92%, the outer ~4% ring of its layout frame maps to a
bounds coordinate *outside* `bounds`, so `point(inside:)` returns `false` and the touch
is routed past the button. `touchesBegan` never fires and the tap is silently dropped —
not delayed, **dropped**.

During fast typing the off-beat taps (2nd, 4th, 6th…) land while the previous tap's key
is still mid-squish, so they fall in that dead ring. This is the same mechanism behind
the original "double-tap gives one `t`".

### Fix

`GlassKeyButton` overrides `point(inside:with:)` to test against the **full un-squished
frame**, expanded by the exact inverse of the maximum scale:

```swift
override func point(inside point: CGPoint, with event: UIEvent?) -> Bool {
    let minScale = 1 - Self.pressScaleDepth        // 0.92
    let marginFraction = 0.5 / minScale - 0.5      // ~0.0435
    let expanded = bounds.insetBy(
        dx: -bounds.width * marginFraction,
        dy: -bounds.height * marginFraction
    )
    return expanded.contains(point)
}
```

The squish depth is hoisted into `static let pressScaleDepth: CGFloat = 0.08` and used by
both `applySquish` and this override, so the visual scale and the hit-test compensation
can never drift apart. The ~4.3% expansion is well inside the 6 pt inter-key gap, so keys
do not steal touches from their neighbours at rest.

Proven by `GlassKeyButtonTests.test_squishedButton_stillReceivesTouchInOuterRing`: a
button scaled to 0.92 is added to a container, and a `hitTest` at a point in the dead ring
must return the button. RED before the override, GREEN after.

## Root cause 2 — slow release animation (sluggish feel)

With taps no longer dropped, the residual sluggishness was the squish animation itself.
The press took 80 ms and the **release took 220 ms** — a 300 ms cycle. During fast typing
the previous key was still growing back while the next key was already being pressed, so
the visual feedback visibly trailed the finger even though every character had registered.

### Fix

Shorten the durations so the release finishes before the next keystroke. `GlassKeyPressAnimator`
now exposes:

```swift
static let pressDuration: TimeInterval = 0.040    // was 0.080
static let releaseDuration: TimeInterval = 0.090  // was 0.220
```

Full cycle drops from 300 ms to 130 ms. The press is near-instant for immediate tactile
feedback; the release keeps up with rapid taps. The instant colour highlight on press
(`pressedBackground` in `KeyStyle.updateGlassAppearance`) is unchanged, so press feedback
stays clear. Timing is asserted by `GlassKeyPressAnimatorTests`.

## Supporting changes (touch routing)

These landed alongside the two root-cause fixes and remove other ways a fast tap could be
missed or feel delayed:

- **`isMultipleTouchEnabled = true` on `GlassKeyButton`.** With the UIKit default
  (`false`), a second overlapping touch is dropped by UIKit *before* `touchesBegan` is
  even called. Enabling it lets every simultaneous touch through.
- **Letter/symbol keys fire via `onPress` in `touchesBegan`, not `.touchUpInside`.**
  `UIControl`'s single-touch tracking does not deliver `.touchDown` for a second touch
  while another is active. The `onPress` closure fires directly from `touchesBegan`,
  bypassing `UIControl`'s event system, so every physical press registers a character.
  (See `KeyboardLayerFactory.makeLetterKey` / `makeSymbolKey`.)
- **Backspace deletes on press-down.** `BackspaceButton` fires `onTap` from `touchesBegan`
  instead of `touchesEnded`, removing the press-duration delay before a character
  disappears.
- **Optimistic render + generation coalescing** in `KeyboardInputHandler` (`sendChar`,
  `backspaceTapped`): the strip's roman hint updates immediately from the last known
  state, and only the most recent in-flight session render is applied
  (`sendGeneration` / `backspaceGeneration`), so rapid taps never queue a backlog of
  stale UI updates on the main thread.

## Verification

- All 19 Swift test suites pass (`xcodebuild test -scheme KhmerIMEKeyboardTests`).
- On-device (iPhone X, Release, Full Access): double-tap reliably yields `tt`, rapid
  `tttttttt` loses no characters, and keys no longer trail the finger.

## If latency returns

The next suspect, untouched here, is the live `UIVisualEffectView` blur on every key —
re-compositing Metal blur on each press colour change is expensive on the A11 GPU. The
mitigation would be a flat fill (drop the live blur) on press, or only on older devices.
Try this before reaching for the data/decoder path; the symptoms above were entirely in
the view/touch layer.
