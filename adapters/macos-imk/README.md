# khmerime-macos-imk

macOS InputMethodKit adapter. Rust core exposed to Swift via UniFFI; Swift host shell uses `IMKInputController` + `NSPanel` for the candidate UI.

## Architecture

```
NSEvent (key press)
  → KhmerInputController.handle(_:client:)
      → MacosIMKSession.handleEvent(keyval, macKeycode, modifierFlags)   [Rust/UniFFI]
          → khmerime_session (core transliteration)
      ← MacosRenderState { preedit, candidates, segments, commitText, consumed }
  → setMarkedText / insertText / CandidatePanel.update()
```

Generated UniFFI bindings live in `swift/KhmerIMEMacOS/Generated/` — do not edit by hand.

## Prerequisites

| Tool | Install |
|------|---------|
| Rust stable | `rustup update stable` |
| x86_64 target | `rustup target add x86_64-apple-darwin` |
| `xcodegen` | `brew install xcodegen` |
| Xcode 15+ | App Store |
| Apple ID in Xcode | Xcode → Settings → Accounts |

## Building

```bash
# From repo root:
make platform-build-macos
```

This runs: `cargo build` (arm64 + x86_64) → `lipo` universal → `xcodebuild -create-xcframework` → `xcodegen generate`.

## Installing (macOS 26 / Tahoe and later)

macOS 26 enforces Gatekeeper on input methods. Ad-hoc signed apps (`-`) are **rejected** — `spctl --add` was removed in Tahoe. A local Apple Development certificate is enough for Xcode to produce a valid signature, but Gatekeeper can still reject the installed input method if the app is not Developer ID signed and notarized.

### Step 1 — find your Team ID

Open **Xcode → Settings → Accounts**, select your Apple ID, click **Manage Certificates**. The Team ID is the 10-character code in parentheses next to your name (e.g. `AB12CD34EF`).

Or from the terminal:

```bash
defaults read com.apple.dt.Xcode IDEProvisioningTeams 2>/dev/null \
  | grep -E "teamID|providerName"
```

### Step 2 — build and install

```bash
# From repo root — replace AB12CD34EF with your Team ID:
make platform-install-macos DEVELOPMENT_TEAM=AB12CD34EF
```

This auto-detects a matching local Apple Development certificate, signs manually, checks Gatekeeper, copies the `.app` to `~/Library/Input Methods/`, strips quarantine, and runs `lsregister`.

If Gatekeeper rejects the signed app, inspect the exact reason:

```bash
syspolicy_check distribution /tmp/khmerime-macos-build/Build/Products/Release/KhmerIMEMacOS.app
```

For a Gatekeeper-accepted distribution build, use a Developer ID Application certificate and notarize the app with Apple. Apple documents that [Developer ID signing](https://developer.apple.com/help/account/certificates/create-developer-id-certificates/) plus a notarization ticket lets Gatekeeper verify software distributed outside the Mac App Store.

### Step 3 — activate

1. **Log out and log back in** (required — TIS reads input method list at login time).
2. Open **System Settings → Keyboard → Input Sources → `+`**.
3. Search for **Khmer IME** and click Add.
4. Switch to it via the menu bar input source selector (or `⌃Space`).

> If "Khmer IME" still doesn't appear, verify Gatekeeper accepts the build:
> ```bash
> spctl --assess --verbose ~/Library/Input\ Methods/KhmerIMEMacOS.app
> # Should print: accepted
> ```
>
> You can also run:
> ```bash
> make platform-diagnose-macos
> ```

## Iterating (after first install)

```bash
# Rebuild Rust + Swift, reinstall, re-register — no re-login needed for code changes
# (the system restarts the daemon automatically on next input focus):
make platform-install-macos DEVELOPMENT_TEAM=AB12CD34EF
```

Force-restart the daemon if changes don't take effect:

```bash
killall KhmerIMEMacOS 2>/dev/null; true
```

## Running Tests

```bash
cargo test -p khmerime_macos_imk
```

All 27 protocol tests run on macOS without Xcode (pure Rust).

## Key Files

| File | Purpose |
|------|---------|
| `src/lib.rs` | UniFFI-exported `MacosIMKSession` and `MacosRenderState` |
| `swift/KhmerIMEMacOS/KhmerInputController.swift` | IMKInputController subclass — key handling, render loop |
| `swift/KhmerIMEMacOS/CandidatePanel.swift` | Non-activating NSPanel for segments + candidates |
| `swift/KhmerIMEMacOS/main.swift` | IMKServer bootstrap, NSApplication.run() |
| `swift/KhmerIMEMacOS/Info.plist` | Bundle metadata, TIS registration keys |
| `swift/project.yml` | xcodegen spec |

## Debugging Checklist

- `activateServer` / `deactivateServer` must be paired — check with `pgrep KhmerIMEMacOS`
- Key events forwarded exactly once — check `state.consumed` return value
- Commit text emitted via `insertText:replacementRange:`, not `setMarkedText:`
- Panel never steals focus — `NSWindowStyleMask.nonactivatingPanel` must be set
- `spctl --assess` must print `accepted` for Gatekeeper to allow registration
