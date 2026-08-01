# Signing & Store-Upload Pipeline — Phased Plan

Resume-from-here notes for signing each platform's release artifact and getting it
into the right distribution channel. Companion to [PACKAGING.md](../PACKAGING.md)
(which covers the config-file convention). Same rule everywhere: **`*.example` is
committed, the real signing config is git-ignored; absent config → today's unsigned
build, untouched.** Signing runs **locally on the Mac first** (no CI secrets yet).

Legend: ✅ done · 🔒 blocked on you · ⬜ not started.

---

## Phase 1 — macOS: Developer ID signed `.pkg` (direct download)  ✅ wired · 🔒 blocked on installer cert

**Goal:** `make macos-package` emits a signed + notarized + stapled `.pkg` that
passes Gatekeeper (Tahoe silently rejects unsigned input methods).

**Done (code wired, gated):**
- `scripts/platforms/macos/imk/build_pkg.sh` — signs app (Developer ID Application,
  hardened runtime, entitlements) → `productbuild --sign` (Developer ID Installer) →
  `notarytool submit --wait` → `stapler staple` → verifies. Falls back to unsigned
  when either identity is unset/placeholder.
- `Makefile` `macos-package` — passes `MACOS_*` signing vars from
  `macos-signing.local.mk` into the script.
- `adapters/macos-imk/macos-signing.example.mk` — documents `MACOS_INSTALLER_SIGN_IDENTITY`.

**Credentials:**
- ✅ `Developer ID Application` cert (team 9289LTXAT7) — in keychain.
- ✅ `khmerime-notary` notarytool keychain profile — used by the install flow.
- 🔒 **`Developer ID Installer` cert — MISSING, you create it.**

**To finish (≈5 min once cert exists):**
1. Xcode → Settings → Accounts → Manage Certificates → `+` → **Developer ID Installer**.
2. `security find-identity -v | grep "Developer ID Installer"` → copy the SHA-1.
3. Add to `adapters/macos-imk/macos-signing.local.mk`:
   `MACOS_INSTALLER_SIGN_IDENTITY := <sha1>`
4. `make macos-package` → `dist/macos/KhmerIME-<v>-macos.pkg` (no `-unsigned`).

**Done when:** `spctl --assess --type install` and `pkgutil --check-signature` pass on
the emitted pkg; double-clicking installs without Gatekeeper warnings.

**Ship:** build locally, upload the signed `.pkg` to the GitHub release (CI still
emits unsigned — that's expected under local-first).

---

## Phase 2 — iOS: App Store / TestFlight  ✅ wired · 🔒 blocked on Apple Distribution cert

**Goal:** `dist/ios/*.xcarchive` → signed `.ipa` → uploaded to App Store Connect.
Keyboards ship via the App Store only.

**One command:**

```bash
make ios-release
```

It archives signed (`KHMERIME_IOS_SIGN=1`), exports the `.ipa`, then validates and
uploads. Each half is gated by its config file, and an absent config degrades with
instructions instead of failing — the same rule the macOS and Android flows follow:

| Config (git-ignored)                              | Absent → |
|---------------------------------------------------|----------|
| `adapters/ios-keyboard/ExportOptions.plist`        | stop after the archive |
| `adapters/ios-keyboard/appstore-upload.local.sh`   | export the `.ipa`, skip upload |

Both have committed `*.example` templates to copy.

**Credentials to obtain (you):**
- **Apple Distribution** cert — the current blocker. `security find-identity -v -p
  codesigning` shows only `Apple Development` and `Developer ID Application`. Create it
  in Xcode → Settings → Accounts → Manage Certificates → `+` → Apple Distribution.
- **Distribution provisioning profiles for BOTH** the container app *and* the keyboard
  appex — two bundle IDs, both need a profile.
- **App Store Connect API key** for CLI upload: `.p8` + Key ID + Issuer ID, created at
  App Store Connect → Users and Access → Integrations, **App Manager** role. Apple
  serves the `.p8` once — save and back it up.

**Done when:** `make ios-release` uploads and the build appears in TestFlight
(processing takes a few minutes after upload).

**Gotcha:** the appex profile is the usual failure point — both app and keyboard must
be covered or export fails. `xcodebuild`'s error names the offending bundle ID.

---

## Phase 3 — Android: Google Play  ⬜ not started

**Goal:** signed `.aab` uploaded to Play Console.

**Current state:**
- `scripts/platforms/android/ime/build_aab.sh` builds an **unsigned** `.aab`
  (`:app:bundleRelease`, no `signingConfig`). Play re-signs with its own app key, but
  the **upload** must be signed with *your upload key* — that's the gap.

**Credentials to obtain (you):**
- **Upload keystore** (`.keystore` + passwords) — enroll in Play App Signing; this is
  the *upload* key, not the app-signing key (Google holds that).
- **Play Developer API service account** JSON — for automated upload.

**Files to touch:**
- Copy `adapters/android-ime/keystore.properties.example` → `keystore.properties` (git-ignored).
- `adapters/android-ime/app/build.gradle(.kts)` — add a `signingConfig` that reads
  `keystore.properties`, wire it into the `release` build type (gated: no file → unsigned, as today).
- Upload step: Gradle Play Publisher plugin, or `fastlane supply`, or the Play Developer
  API directly — auth via the service-account JSON. Gate on the JSON being present.

**Done when:** `make android-package` emits an upload-key-signed `.aab` that Play Console
accepts on upload.

---

## Phase 4 — Windows: Authenticode `.msi`  ⬜ not started (lower priority)

`scripts/platforms/windows/tsf/build_msi.ps1` emits an unsigned `.msi`;
`adapters/windows-tsf/signing.example.ps1` holds the pattern. Sign post-build with
`signtool` using a cert thumbprint or `.pfx`. Needs a code-signing cert (EV or OV).

---

## Later — CI signing (deferred by choice)

All of the above run locally first. To automate in `release-candidate.yml`, export the
key material as GitHub secrets (base64 `.p12`/keystore/`.p8` + passwords), import into an
ephemeral keychain per job, and pass the same identities. Keep every step gated so a
missing secret still yields the unsigned build. Do this only once the local flows are proven.
