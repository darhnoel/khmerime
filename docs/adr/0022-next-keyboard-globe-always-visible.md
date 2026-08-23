# 0022 — The next-keyboard globe is always visible (iOS + Android)

Status: accepted (2026-08-23)

## Context

App Store review **rejected KhmerIME 1.0 under Guideline 4.4.1** (Design –
Extensions): *"Your keyboard extension does not provide a way for users to
switch to another keyboard."* Reviewed on iPad Pro 11" (M4). Users had also
reported the switch button missing on iPhone X and other larger phones, while
it appeared on iPhone SE.

**Root cause (iOS).** The bottom row shipped an "Option B" slot-sharing hack:
the globe key (tag 999) and an "EN" English-layer toggle (tag 998) occupied one
slot, and `viewWillLayoutSubviews` showed exactly one based on
`needsInputModeSwitchKey`:

```
let show = needsInputModeSwitchKey
globe.isHidden = !show   // hidden when false
en.isHidden    = show    // EN shown instead
```

`needsInputModeSwitchKey` returns `false` on many devices/contexts (iPad,
iPhone X-class, floating/hardware situations) — it is a hint about the system
*possibly* offering switching, NOT a guarantee the user has any other way to
switch. When false, the globe was replaced by the EN key and there was **no
next-keyboard button at all**. That is what Apple rejected and what users saw.

**Root cause (Android).** The Android IME bottom row
(`[En][123][space][.][↵]`) never had a globe/next-keyboard key, and the service
has no IME-switch action at all (`switchToNextInputMethod` /
`shouldOfferSwitchingToNextInputMethod` appear nowhere). Same UX gap as iOS,
one platform worse.

The globe's wiring on iOS is already correct and Apple-recommended — short tap
→ `advanceToNextInputMode()`, long press → `handleInputModeList(from:with:)`
(system picker). Only the visibility gate was wrong.

## Decision

**The next-keyboard globe is always visible on both platforms, never gated on
any device/context signal.**

- **iOS:** remove the `needsInputModeSwitchKey`-based hide. The globe stays
  visible on all devices. The EN key stays visible too (it is a distinct
  feature — the in-keyboard English-layer toggle, not a keyboard switcher), so
  globe and EN both become permanent members of the bottom row. The space bar
  flexes to absorb the width.
- **Android:** add a globe key at the start of the bottom row
  (`[globe][En][123][space][.][↵]`), mirroring iOS. Tap →
  `switchToNextInputMethod`; long press → `showInputMethodPicker`.
- **Sole-keyboard case:** the globe is still shown when KhmerIME is the only
  enabled keyboard (Apple mandates presence). Tap is a harmless no-op; long
  press opens the system picker so the user can still add/switch keyboards.

## Consequences

- Guideline 4.4.1 satisfied: a next-keyboard control is always present, placed
  where the system globe sits.
- iOS bottom row gains one always-visible key (globe + EN both permanent). On
  the narrowest phone (SE) the space bar is a little narrower; acceptable.
- Cross-platform parity: same row shape and globe gesture set on iOS + Android.
- Android gains a real IME-switch affordance it never had.

## Alternatives rejected

- **Keep gating on `needsInputModeSwitchKey`** — the exact behavior Apple
  rejected. It is a system hint, not a switching guarantee.
- **Move the switch onto the "." key on affected devices only** (the first
  instinct) — device-conditional placement re-creates the "sometimes missing"
  inconsistency and is non-standard; a permanent globe is simpler and correct.
- **Show the globe only when other IMEs are enabled** (Android
  `shouldOfferSwitchingToNextInputMethod`) — reintroduces the conditional that
  users reported as "missing"; and long-press-picker needs to work even as the
  sole keyboard.
- **Fold EN into the globe long-press** — EN is a frequently-used layer toggle;
  hiding it behind a gesture harms discoverability. Keep both keys.

## Reversal cost

Low. The change is deleting a visibility gate (iOS) and adding one key + one
action (Android). No data or protocol changes.
